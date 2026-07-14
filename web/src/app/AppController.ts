import type { Viewport } from '../viewport/Viewport';
import type { FileDrop } from '../views/FileDrop';
import type { ConfigPanel } from '../views/ConfigPanel';
import type { ScorePanel } from '../views/ScorePanel';
import type { CandidateList } from '../views/CandidateList';
import { AppState } from './AppState';
import type { OriData, Candidate, ComputeConfig, WorkerMessage, WorkerRequest } from '../types';
import { defaultConfig } from '../types';
import { initWasm, loadSTLBytes, prepareData } from '../loadSTL';
import { decimateForScore } from '../compute';
import { applyConvention } from '../convention';
import type { LoadConvention } from '../convention';
import { dirFromQuat } from '../quaternion';
import { score_direction, compute_norm_bounds as wasm_compute_norm_bounds } from '../../pkg/orient_core.js';
import { WEIGHT_PRESETS } from '../profiles';
import { exportSTL } from '../exportSTL';
import { rotatePositions } from '../rotate';
import { DECIMATE_TARGET, STORAGE_KEY, SCHEMA_VERSION, MIN_ANGLE_DEG, DEFAULT_PROFILE, DEFAULT_RANKER } from '../constants';
import { PROFILE_LABELS, RANKER_LABELS, RANKER_HINTS } from '../views/ConfigPanel';

// ── Dependency Injection Contract ──

export interface AppControllerDeps {
  state: AppState;
  viewport: Viewport;
  fileDrop: FileDrop;
  configPanel: ConfigPanel;
  scorePanel: ScorePanel;
  candidateList: CandidateList;
  workerFactory: () => Worker;
  statusEl: HTMLElement;
  progressContainer: HTMLElement;
  progressBar: HTMLElement;
  progressLabel: HTMLElement;
  resultsPlaceholder: HTMLElement;
  panelRight: HTMLElement;
  cancelBtn: HTMLButtonElement;
}

const paint = () => new Promise<void>((r) => setTimeout(r, 0));

export function bboxDiagonalFromPositions(pos: Float32Array): number {
  let minX = Infinity, minY = Infinity, minZ = Infinity;
  let maxX = -Infinity, maxY = -Infinity, maxZ = -Infinity;
  for (let i = 0; i < pos.length; i += 3) {
    const x = pos[i], y = pos[i + 1], z = pos[i + 2];
    if (x < minX) minX = x; if (x > maxX) maxX = x;
    if (y < minY) minY = y; if (y > maxY) maxY = y;
    if (z < minZ) minZ = z; if (z > maxZ) maxZ = z;
  }
  const dx = maxX - minX, dy = maxY - minY, dz = maxZ - minZ;
  return Math.sqrt(dx * dx + dy * dy + dz * dz);
}

interface StoredConfig {
  version: number; profile: string; ranker: string;
  criticalAngleDeg: number; convention: string; hullSphere: boolean;
}

export class AppController {
  private lastFile: File | null = null;
  private lastFileBytes: Uint8Array | null = null;
  private currentWorker: Worker | null = null;
  private _liveRegionEl: HTMLElement | null = null;

  private get liveRegionEl(): HTMLElement | null {
    if (!this._liveRegionEl) {
      this._liveRegionEl = document.getElementById("status-live");
    }
    return this._liveRegionEl;
  }

  constructor(private deps: AppControllerDeps) {
    deps.fileDrop.onFile((f) => this.handleFile(f));

    deps.configPanel.onChange(() => {
      const angle = deps.configPanel.getAngle();
      const hullSphere = deps.configPanel.getHullSphere();
      const convention = deps.configPanel.getConvention();
      const profile = deps.configPanel.getProfile();
      const ranker = deps.configPanel.getRanker();
      const oldProfile = deps.state.get('currentProfile');
      const config = deps.state.get('config');
      const oldConvention = deps.state.get('loadConvention');

      config.criticalAngleDeg = angle;
      config.mode = hullSphere ? 'hull_plus_sphere' : 'hull';
      deps.state.set('config', config);
      deps.state.set('currentProfile', profile);
      deps.state.set('currentRanker', ranker);
      deps.viewport.setCriticalAngle(angle);

      if (convention !== oldConvention) deps.state.set('loadConvention', convention);

      if (profile !== oldProfile) {
        const lod = deps.state.get('lastOriData');
        if (deps.state.get('candidates').length > 0 && lod) this.spawnCompute(lod);
      }

      this.saveConfig();
      this.markDirty();
      if (deps.state.get('liveData')) this.updateLiveScore(deps.viewport.getMeshQuaternion());
    });

    deps.configPanel.onRecalc(() => this.recalculate());
    deps.configPanel.onFind(() => {
      const lod = deps.state.get('lastOriData');
      if (lod) this.spawnCompute(lod);
    });
    deps.configPanel.onExport(() => this.exportCurrent());

    deps.candidateList.onSelect((i) => this.showCandidate(i));

    deps.state.subscribe((key) => {
      if (key === 'candidates') {
        deps.candidateList.render(deps.state.get('candidates'), deps.state.get('currentIndex'));
      }
    });

    deps.cancelBtn.addEventListener('click', () => this.cancelCompute());
  }

  async boot(): Promise<void> {
    this.deps.statusEl.textContent = 'Initializing WASM...';
    this.loadConfig();
    this.deps.viewport.setCriticalAngle(this.deps.state.get('config').criticalAngleDeg);
    try {
      await initWasm();
      this.deps.statusEl.textContent = 'Ready \u2014 load an STL file';
    } catch (err) {
      this.deps.statusEl.textContent = 'WASM init failed: ' + err;
      console.error(err);
    }
  }

  async handleFile(file: File): Promise<void> {
    this.cancelCompute();
    this.lastFile = file;
    if (!file.name.toLowerCase().endsWith('.stl')) {
      this.deps.statusEl.textContent = 'Please select a .stl file';
      return;
    }
    this.deps.progressContainer.style.display = 'block';
    this.deps.progressLabel.textContent = 'Reading file...';
    this.deps.progressBar.className = 'progress-bar-fill indeterminate';
    try {
      const bytes = await loadSTLBytes(file);
      this.lastFileBytes = bytes;
      this.deps.state.set('stlName', file.name);

      this.deps.progressLabel.textContent = 'Parsing STL (WASM)...';
      await paint();
      const fullData = this.parseCurrentData();
      if (!fullData) throw new Error('No triangles or candidates in STL');

      this.deps.state.set('lastOriData', fullData);

      this.deps.progressLabel.textContent = 'Rendering model...';
      this.deps.statusEl.textContent = 'Rendering model...';
      await paint();
      this.deps.viewport.loadModel(fullData.positions, fullData.normals);
      this.deps.viewport.resetCamera();
      await paint();

      const diag = bboxDiagonalFromPositions(fullData.positions);
      this.deps.state.set('bboxDiagonal', diag);

      const decimated = decimateForScore(fullData, DECIMATE_TARGET);
      this.deps.state.set('liveData', { positions: decimated.positions, normals: decimated.normals, areas: decimated.areas });
      this.computeNormBounds(fullData);
      this.deps.viewport.enterOverlayMode((q) => this.updateLiveScore(q));

      this.deps.resultsPlaceholder.style.display = 'none';
      this.deps.panelRight.style.display = 'block';
      this.updateLiveScore(this.deps.viewport.getMeshQuaternion());

      this.deps.progressContainer.style.display = 'none';
      this.deps.statusEl.textContent = 'Drag the rings to rotate. Click "Find Candidates" for suggestions.';
      this.markClean();
      this.deps.candidateList.hide();
      this.deps.configPanel.enableFind(true);
      this.deps.configPanel.enableExport(true);
      this.deps.configPanel.enableRecalc(false);
    } catch (err) {
      this.deps.progressContainer.style.display = 'none';
      this.deps.statusEl.textContent = 'Error: ' + err;
      console.error(err);
    }
  }

  // ── Internal ──

  private parseCurrentData(): OriData | null {
    if (!this.lastFileBytes) return null;
    const config = this.deps.state.get('config');
    const raw = prepareData(this.lastFileBytes, config as unknown as Record<string, unknown>) as unknown as {
      positions: number[]; normals: number[]; areas: number[]; directions: number[];
    };
    if (raw.positions.length === 0 || raw.directions.length === 0) return null;
    const conv = this.deps.state.get('loadConvention');
    return {
      positions: applyConvention(new Float32Array(raw.positions), conv),
      normals: applyConvention(new Float32Array(raw.normals), conv),
      areas: new Float32Array(raw.areas),
      directions: applyConvention(new Float32Array(raw.directions), conv),
    };
  }

  private computeNormBounds(data: OriData): void {
    const liveData = this.deps.state.get('liveData');
    if (!liveData) return;
    try {
      const raw = wasm_compute_norm_bounds(
        liveData.positions, liveData.normals, liveData.areas,
        data.directions, this.deps.state.get('config').criticalAngleDeg,
      ) as Float32Array;
      this.deps.state.set('normBounds', { lo: Array.from(raw.subarray(0, 5)), hi: Array.from(raw.subarray(5, 10)) });
    } catch (err) {
      this.deps.state.set('normBounds', null);
      console.warn('computeNormBounds failed, scoring will be degraded', err);
    }
  }

  private updateLiveScore(q: [number, number, number, number]): void {
    const liveData = this.deps.state.get('liveData');
    const normBounds = this.deps.state.get('normBounds');
    if (!liveData || !normBounds) return;

    const dir = dirFromQuat(q);
    const config = this.deps.state.get('config');
    let raw: Float32Array;
    try {
      raw = score_direction(liveData.positions, liveData.normals, liveData.areas,
        dir[0], dir[1], dir[2], config.criticalAngleDeg, config.refineIterations ?? 0);
    } catch (err) {
      raw = new Float32Array(8);
      console.warn('score_orientation failed, using fallback', err);
    }
    const [, , , overhang, foot, cross, surf, height] = raw;

    const { lo, hi } = normBounds;
    const clamp = (v: number) => v < 0 ? 0 : v > 1 ? 1 : v;
    const span = (i: number) => Math.max(hi[i] - lo[i], 1e-9);
    const bboxDiag = this.deps.state.get('bboxDiagonal');
    const heightCost = clamp((height - lo[4]) / span(4));
    const costs = [
      clamp((overhang - lo[0]) / span(0)), clamp((foot - lo[1]) / span(1)),
      clamp((cross - lo[2]) / span(2)), clamp((hi[3] - surf) / span(3)),
      heightCost,
    ];

    const profile = this.deps.state.get('currentProfile');
    const w = WEIGHT_PRESETS[profile] ?? WEIGHT_PRESETS[DEFAULT_PROFILE];
    const weights = [w.wOverhang, w.wFootprint, w.wCross, w.wSurface, w.wHeight];

    const ranker = this.deps.state.get('currentRanker');
    let score: number;
    const wSum = weights.reduce((a, b) => a + b, 0);
    const maxW = Math.max(...weights);
    if (ranker === 'consensus') {
      score = maxW > 0 ? 1 - costs.reduce((acc, c, i) => Math.max(acc, weights[i] * c), 0) / maxW : 1 - costs[0];
    } else {
      score = wSum > 0 ? 1 - costs.reduce((acc, c, i) => acc + weights[i] * c, 0) / wSum : 1 - costs[0];
    }
    this.deps.scorePanel.update({
      score,
      costs,
      weights,
      profileLabel: PROFILE_LABELS[profile] ?? profile,
      rankerLabel: RANKER_LABELS[ranker] ?? ranker,
      hint: RANKER_HINTS[ranker] ?? '',
    });

    const profileLabel = PROFILE_LABELS[profile] ?? profile;
    const pct = (score * 100).toFixed(0);
    if (this.liveRegionEl) {
      this.liveRegionEl.textContent = `Orientation score ${pct}%, Profile: ${profileLabel}`;
    }
  }

  private async spawnCompute(data: OriData): Promise<void> {
    this.deps.progressContainer.style.display = 'block';
    this.deps.progressBar.className = 'progress-bar-fill indeterminate';
    this.deps.progressLabel.textContent = 'Decimating mesh...';
    this.deps.state.set('candidates', []);

    const config = this.deps.state.get('config');
    const computeConfig: ComputeConfig = {
      criticalAngleDeg: config.criticalAngleDeg,
      excludeUnstable: config.excludeUnstable,
      maxCandidates: config.maxCandidates,
      refineIterations: config.refineIterations,
    };

    await paint();
    const decimated = decimateForScore(data, DECIMATE_TARGET);
    this.deps.state.set('liveData', { positions: decimated.positions, normals: decimated.normals, areas: decimated.areas });
    this.deps.progressLabel.textContent = 'Computing candidates...';
    const profile = this.deps.state.get('currentProfile');
    const weights = WEIGHT_PRESETS[profile] ?? WEIGHT_PRESETS[DEFAULT_PROFILE];
    const wArr: [number, number, number, number, number] = [weights.wOverhang, weights.wFootprint, weights.wCross, weights.wSurface, weights.wHeight];

    const worker = this.deps.workerFactory();
    this.currentWorker = worker;

    worker.onmessage = (e: MessageEvent<WorkerMessage>) => {
      const msg = e.data;
      switch (msg.type) {
        case 'progress':
          this.deps.progressBar.className = 'progress-bar-fill determinate';
          this.deps.progressBar.style.width = msg.value + '%';
          break;
        case 'results': {
          const merged = msg.candidates;
          if (merged.length > 0) {
            merged.sort((a, b) => b.compositeScore - a.compositeScore);
            this.deps.state.set('candidates', merged);
            this.deps.state.set('currentIndex', 0);
          }
          this.finishCompute();
          break;
        }
        case 'error':
          console.error('Worker error:', msg.message);
          this.finishCompute();
          break;
      }
    };
    worker.onerror = (err) => {
      console.error('Worker error:', err);
      this.finishCompute();
    };
    const nb = this.deps.state.get('normBounds');
    worker.postMessage({
      data: decimated, config: computeConfig, weights: wArr,
      ranker: this.deps.state.get('currentRanker'),
      maxCandidates: computeConfig.maxCandidates, minAngleDeg: MIN_ANGLE_DEG,
      normLo: nb?.lo ?? null, normHi: nb?.hi ?? null,
    } satisfies WorkerRequest);
  }

  private finishCompute(): void {
    this.deps.progressContainer.style.display = 'none';
    this.currentWorker = null;
    this.markClean();
    const candidates = this.deps.state.get('candidates');
    if (candidates.length === 0) {
      this.deps.statusEl.textContent = 'No candidates generated';
      return;
    }
    this.deps.statusEl.textContent = candidates.length + ' candidates \u2014 click one to try it';
    this.deps.state.set('currentIndex', 0);
    this.deps.viewport.setCriticalAngle(this.deps.state.get('config').criticalAngleDeg);
    this.deps.viewport.showCandidate(candidates[0].quaternion);
    this.updateLiveScore(this.deps.viewport.getMeshQuaternion());
  }

  cancelCompute(): void {
    if (this.currentWorker) { this.currentWorker.terminate(); this.currentWorker = null; }
    this.deps.progressContainer.style.display = 'none';
  }

  showCandidate(index: number): void {
    const candidates = this.deps.state.get('candidates');
    if (index < 0 || index >= candidates.length) return;
    this.deps.state.set('currentIndex', index);
    this.deps.viewport.setCriticalAngle(this.deps.state.get('config').criticalAngleDeg);
    this.deps.viewport.showCandidate(candidates[index].quaternion);
    this.updateLiveScore(this.deps.viewport.getMeshQuaternion());
    this.deps.candidateList.render(candidates, index);
  }

  private async recalculate(): Promise<void> {
    if (!this.lastFileBytes) return;
    this.cancelCompute();
    this.deps.progressContainer.style.display = 'block';
    this.deps.progressLabel.textContent = 'Reparsing STL...';
    this.deps.progressBar.className = 'progress-bar-fill indeterminate';
    await paint();
    const data = this.parseCurrentData();
    if (!data) {
      this.deps.statusEl.textContent = 'No data to recalculate';
      this.deps.progressContainer.style.display = 'none';
      return;
    }
    this.deps.state.set('lastOriData', data);
    const decimated = decimateForScore(data, DECIMATE_TARGET);
    this.deps.state.set('liveData', { positions: decimated.positions, normals: decimated.normals, areas: decimated.areas });
    this.deps.viewport.setCriticalAngle(this.deps.state.get('config').criticalAngleDeg);
    this.computeNormBounds(data);
    this.updateLiveScore(this.deps.viewport.getMeshQuaternion());
    if (this.deps.state.get('candidates').length > 0) {
      this.spawnCompute(data);
    } else {
      this.deps.progressContainer.style.display = 'none';
      this.markClean();
    }
  }

  private exportCurrent(): void {
    const lod = this.deps.state.get('lastOriData');
    if (!lod) return;
    const q = this.deps.viewport.getMeshQuaternion();
    exportSTL(rotatePositions(lod.positions, q), this.deps.state.get('stlName'), this.deps.state.get('currentIndex') + 1);
  }

  // ── Config Persistence ──

  private saveConfig(): void {
    const data: StoredConfig = {
      version: SCHEMA_VERSION,
      profile: this.deps.state.get('currentProfile'),
      ranker: this.deps.state.get('currentRanker'),
      criticalAngleDeg: this.deps.state.get('config').criticalAngleDeg,
      convention: this.deps.state.get('loadConvention'),
      hullSphere: this.deps.configPanel.getHullSphere(),
    };
    try { localStorage.setItem(STORAGE_KEY, JSON.stringify(data)); } catch (err) {
      this.deps.statusEl.textContent = 'Warning: Could not save preferences (storage full or private mode)';
      console.warn('saveConfig failed', err);
    }
  }

  private loadConfig(): void {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (!raw) return;
      const data = JSON.parse(raw) as StoredConfig;
      if (data.version !== SCHEMA_VERSION) { localStorage.removeItem(STORAGE_KEY); return; }
      this.deps.state.set('currentProfile', data.profile ?? DEFAULT_PROFILE);
      this.deps.state.set('currentRanker', data.ranker ?? DEFAULT_RANKER);
      const config = this.deps.state.get('config');
      config.criticalAngleDeg = data.criticalAngleDeg ?? 30;
      this.deps.state.set('config', config);
      this.deps.state.set('loadConvention', (data.convention as LoadConvention) ?? 'z-up');
      this.deps.configPanel.setHullSphere(data.hullSphere ?? false);
      this.deps.configPanel.setAngle(config.criticalAngleDeg);
      this.deps.configPanel.setConvention((data.convention as LoadConvention) ?? 'z-up');
      this.deps.configPanel.setProfile(data.profile ?? DEFAULT_PROFILE);
      this.deps.configPanel.setRanker(data.ranker ?? DEFAULT_RANKER);
      this.deps.viewport.setCriticalAngle(config.criticalAngleDeg);
    } catch (err) {
      localStorage.removeItem(STORAGE_KEY);
      console.warn('Stored config corrupt, resetting to defaults', err);
    }
  }

  private markDirty(): void {
    this.deps.state.set('isComputeDirty', true);
    this.deps.configPanel.enableRecalc(true);
  }

  private markClean(): void {
    this.deps.state.set('isComputeDirty', false);
    this.deps.configPanel.enableRecalc(false);
  }
}
