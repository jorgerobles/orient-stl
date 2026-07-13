import { initWasm, loadSTLBytes, prepareData } from './loadSTL';
import { Viewport } from './viewport';
import { exportSTL } from './exportSTL';
import { rotatePositions } from './rotate';
import { applyConvention } from './convention';
import type { LoadConvention } from './convention';
import { decimateForScore, WEIGHT_PRESETS } from './compute';
import type { OriData, Candidate, ComputeConfig } from './compute';
import { score_orientation, compute_norm_bounds as wasm_compute_norm_bounds } from '../pkg/orient_core.js';
import { defaultConfig } from './types';
import { dirFromQuat } from './quaternion';
import { nearestCandidateScore } from './nearestScore';

let config = defaultConfig();
let candidates: Candidate[] = [];
let positions: Float32Array | null = null;
let faceNormals: Float32Array | null = null;
let areas: Float32Array | null = null;
let currentIndex = 0;
let stlName = '';
let viewport: Viewport;
let currentWorker: Worker | null = null;
let lastFile: File | null = null;
let lastFileBytes: Uint8Array | null = null;
let loadConvention: LoadConvention = 'z-up';

// Normalization bounds for live scoring: [overhang, footprint, cross, surface, height].
// Populated from sampled directions on load, updated from candidates when computed.
let normBounds: { lo: number[]; hi: number[] } | null = null;

function parseCurrentData(): OriData | null {
  if (!lastFileBytes) return null;
  const raw = prepareData(lastFileBytes, config as unknown as Record<string, unknown>) as unknown as {
    positions: number[]; normals: number[]; areas: number[]; directions: number[];
  };
  if (raw.positions.length === 0 || raw.directions.length === 0) return null;
  return {
    positions: applyConvention(new Float32Array(raw.positions), loadConvention),
    normals: applyConvention(new Float32Array(raw.normals), loadConvention),
    areas: new Float32Array(raw.areas),
    directions: applyConvention(new Float32Array(raw.directions), loadConvention),
  };
}

/** Sample ~30 directions and compute their 5 metrics to establish normalization
 *  bounds for live scoring. Uses WASM `compute_norm_bounds` for cheap sampling. */
function computeNormBounds(data: OriData): void {
  if (!liveData) return;
  try {
    const raw = wasm_compute_norm_bounds(
      liveData.positions, liveData.normals, liveData.areas,
      data.directions, config.criticalAngleDeg,
    ) as Float32Array;
    // raw layout: [lo[0], lo[1], lo[2], lo[3], lo[4], hi[0], hi[1], hi[2], hi[3], hi[4]]
    normBounds = { lo: Array.from(raw.subarray(0, 5)), hi: Array.from(raw.subarray(5, 10)) };
  } catch { /* keep existing normBounds if sample fails */ }
}

const paint = () => new Promise<void>(r => setTimeout(r, 0));

const statusEl = document.getElementById('status')!;
const fileInput = document.getElementById('file-input') as HTMLInputElement;
const dropZone = document.getElementById('drop-zone')!;
const resultsEl = document.getElementById('results')!;
const viewportContainer = document.getElementById('viewport')!;
const progressContainer = document.getElementById('progress-container')!;
const progressBar = document.getElementById('progress-bar')!;
const progressLabel = document.getElementById('progress-label')!;
const cancelBtn = document.getElementById('cancel-btn')!;
const panelLeft = document.getElementById('panel-left')!;
const panelRight = document.getElementById('results')!;
const scoreBig = document.getElementById('score-big')!;
const spProfile = document.getElementById('sp-profile')!;
const spRanker = document.getElementById('sp-ranker')!;
const spRows = document.getElementById('sp-rows')!;
const spHint = document.getElementById('sp-hint')!;
const findBtn = document.getElementById('find-btn') as HTMLButtonElement;
const exportBtn = document.getElementById('export-btn') as HTMLButtonElement;
const candidatesSection = document.getElementById('candidates-section')!;
const candidateList = document.getElementById('candidate-list')!;
const resultsPlaceholder = document.getElementById('results-placeholder')!;

viewport = new Viewport(viewportContainer);

function updateLiveScore(q: [number, number, number, number]): void {
  if (!liveData || !normBounds) return;
  const dir = dirFromQuat(q);

  // Single WASM call: refine + all 5 metrics for the refined direction.
  const { positions: lp, normals: ln, areas: la } = liveData;
  let raw: Float32Array;
  try {
    raw = score_orientation(lp, ln, la, dir[0], dir[1], dir[2], config.criticalAngleDeg, config.refineIterations ?? 0, 42);
  } catch {
    raw = new Float32Array(8);
  }
  const [, , , overhang, foot, cross, surf, height] = raw;

  // Display costs (for the metric bars) — min-max normalized via normBounds.
  const { lo, hi } = normBounds;
  const clamp = (v: number) => v < 0 ? 0 : v > 1 ? 1 : v;
  const span = (i: number) => Math.max(hi[i] - lo[i], 1e-9);
  const costs = [
    clamp((overhang - lo[0]) / span(0)),
    clamp((foot - lo[1]) / span(1)),
    clamp((cross - lo[2]) / span(2)),
    clamp((hi[3] - surf) / span(3)),
    clamp((height - lo[4]) / span(4)),
  ];

  // Overall score: use nearest precomputed candidate's compositeScore,
  // or weighted average of normalized costs as fallback.
  const w = WEIGHT_PRESETS[currentProfile] ?? WEIGHT_PRESETS['resin-biased'];
  const weights = [w.wOverhang, w.wFootprint, w.wCross, w.wSurface, w.wHeight];
  let score: number;
  if (candidates.length > 0) {
    score = nearestCandidateScore([q[0], q[1], q[2], q[3]], candidates).score;
  } else {
    const wSum = weights.reduce((a, b) => a + b, 0);
    score = wSum > 0
      ? 1 - costs.reduce((acc, c, i) => acc + weights[i] * c, 0) / wSum
      : 1 - costs[0];
  }

  scoreBig.textContent = `${(score * 100).toFixed(0)}%`;

  // Populate the score breakdown panel.
  spProfile.textContent = PROFILE_LABELS[currentProfile] ?? currentProfile;
  spRanker.textContent = RANKER_LABELS[currentRanker] ?? currentRanker;
  const qColor = (q: number) => q > 0.6 ? '#27ae60' : q > 0.3 ? '#f0ad4e' : '#e74c3c';
  const names = ['Overhang', 'Footprint', 'Cross-sect', 'Surface', 'Height'];
  const descs = [
    'Faces needing supports (lower=better)',
    'Base contact area (lower=better)',
    'Max layer material — peel force (lower=better)',
    'Axis misalignment — resin finish (higher=better)',
    'Print height — layers & time (lower=better)',
  ];
  spRows.innerHTML = costs.map((cost, i) => {
    const quality = (1 - cost) * 100;
    const wt = weights[i];
    return `<div class="sp-row" title="${descs[i]}">
      <span class="sp-name">${names[i]}</span>
      <div class="sp-bar"><div class="sp-bar-fill" style="width:${quality.toFixed(0)}%;background:${qColor(1 - cost)}"></div></div>
      <span class="sp-pct">${quality.toFixed(0)}%</span>
      <span class="sp-weight">${wt > 0 ? '×' + wt.toFixed(1) : '—'}</span>
    </div>`;
  }).join('');
  spHint.textContent = RANKER_HINTS[currentRanker] ?? '';
}

async function boot(): Promise<void> {
  statusEl.textContent = 'Initializing WASM...';
  loadConfig();
  viewport.setCriticalAngle(config.criticalAngleDeg);
  try {
    await initWasm();
    statusEl.textContent = 'Ready — load an STL file';
  } catch (err) {
    statusEl.textContent = `WASM init failed: ${err}`;
    console.error(err);
  }
}

async function handleFile(file: File): Promise<void> {
  cancelCompute();
  lastFile = file;
  if (!file.name.toLowerCase().endsWith('.stl')) {
    statusEl.textContent = 'Please select a .stl file';
    return;
  }
  progressContainer.style.display = 'block';
  progressLabel.textContent = 'Reading file...';
  progressBar.className = 'progress-bar-fill indeterminate';
  try {
    const bytes = await loadSTLBytes(file);
    lastFileBytes = bytes;
    stlName = file.name;

    progressLabel.textContent = 'Parsing STL (WASM)...';
    await paint();
    const fullData = parseCurrentData();
    if (!fullData) throw new Error('No triangles or candidates in STL');

    positions = fullData.positions;
    faceNormals = fullData.normals;
    areas = fullData.areas;
    lastOriData = fullData;

    progressLabel.textContent = 'Rendering model...';
    statusEl.textContent = 'Rendering model...';
    await paint();
    viewport.loadModel(positions, faceNormals);
    viewport.resetCamera();
    await paint();

    // Enable drag-to-rotate with live scoring (always on, no overlay).
    const decimated = decimateForScore(fullData, 12000);
    liveData = { positions: decimated.positions, normals: decimated.normals, areas: decimated.areas };
    computeNormBounds(fullData);
    viewport.enterOverlayMode(updateLiveScore);

    // Show right panel with initial score for default orientation.
    resultsPlaceholder.style.display = 'none';
    panelRight.style.display = 'block';
    updateLiveScore(viewport.getMeshQuaternion());

    progressContainer.style.display = 'none';
    statusEl.textContent = 'Drag the rings to rotate. Click "Find Candidates" for suggestions.';
    findBtn.disabled = false;
    exportBtn.disabled = false;
    candidates = [];
    candidatesSection.style.display = 'none';
  } catch (err) {
    progressContainer.style.display = 'none';
    statusEl.textContent = `Error: ${err}`;
    console.error(err);
  }
}

let currentProfile: string = 'resin-biased';
let currentRanker: string = 'consensus';
let isComputeDirty = false;
let lastOriData: OriData | null = null;
let liveData: { positions: Float32Array; normals: Float32Array; areas: Float32Array } | null = null;

const recalcBtn = document.getElementById('recalc-btn') as HTMLButtonElement;

const STORAGE_KEY = 'orient-stl-config';
const SCHEMA_VERSION = 1;
interface StoredConfig {
  version: number; profile: string; ranker: string;
  criticalAngleDeg: number; convention: string; hullSphere: boolean;
}

function saveConfig(): void {
  const data: StoredConfig = {
    version: SCHEMA_VERSION,
    profile: currentProfile, ranker: currentRanker,
    criticalAngleDeg: config.criticalAngleDeg,
    convention: loadConvention, hullSphere: hullSphereToggle.checked,
  };
  try { localStorage.setItem(STORAGE_KEY, JSON.stringify(data)); } catch { /* quota */ }
}

function loadConfig(): void {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return;
    const data = JSON.parse(raw) as StoredConfig;
    if (data.version !== SCHEMA_VERSION) { localStorage.removeItem(STORAGE_KEY); return; }
    currentProfile = data.profile ?? 'resin-biased';
    currentRanker = data.ranker ?? 'consensus';
    config.criticalAngleDeg = data.criticalAngleDeg ?? 30;
    loadConvention = data.convention as LoadConvention ?? 'z-up';
    hullSphereToggle.checked = data.hullSphere ?? false;
    config.mode = hullSphereToggle.checked ? 'hull_plus_sphere' : 'hull';
    angleSlider.value = String(config.criticalAngleDeg);
    angleValue.textContent = String(config.criticalAngleDeg);
    conventionSelect.value = loadConvention;
    profileSelect.value = currentProfile;
    rankerSelect.value = currentRanker;
    viewport.setCriticalAngle(config.criticalAngleDeg);
  } catch { /* corrupt → keep defaults */ }
}

function markDirty(): void {
  isComputeDirty = true;
  recalcBtn.disabled = false;
}
function markClean(): void {
  isComputeDirty = false;
  recalcBtn.disabled = true;
}

async function spawnCompute(data: OriData): Promise<void> {
  progressContainer.style.display = 'block';
  progressBar.className = 'progress-bar-fill indeterminate';
  progressLabel.textContent = 'Decimating mesh...';
  candidates = [];

  const computeConfig: ComputeConfig = {
    criticalAngleDeg: config.criticalAngleDeg,
    excludeUnstable: config.excludeUnstable,
    maxCandidates: config.maxCandidates,
    refineIterations: config.refineIterations,
  };

  await paint();
  const decimated = decimateForScore(data, 12000);
  liveData = { positions: decimated.positions, normals: decimated.normals, areas: decimated.areas };
  progressLabel.textContent = 'Computing candidates...';
  const weights = WEIGHT_PRESETS[currentProfile] ?? WEIGHT_PRESETS['resin-biased'];
  const wArr: [number, number, number, number, number] = [weights.wOverhang, weights.wFootprint, weights.wCross, weights.wSurface, weights.wHeight];

  const worker = new Worker(new URL('./orient.worker.ts', import.meta.url), { type: 'module' });
  currentWorker = worker;

  worker.onmessage = (e) => {
    const msg = e.data;
    switch (msg.type) {
      case 'progress':
        progressBar.className = 'progress-bar-fill determinate';
        progressBar.style.width = `${msg.value}%`;
        break;
      case 'results': {
        const merged = msg.candidates as Candidate[];
        if (merged.length > 0) {
          candidates = merged;
          currentIndex = 0;
          normBounds = {
            lo: [
              Math.min(...merged.map((c: Candidate) => c.refinedOverhang)),
              Math.min(...merged.map((c: Candidate) => c.footprint)),
              Math.min(...merged.map((c: Candidate) => c.maxCross)),
              Math.min(...merged.map((c: Candidate) => c.surfaceQuality)),
              Math.min(...merged.map((c: Candidate) => c.estHeight)),
            ],
            hi: [
              Math.max(...merged.map((c: Candidate) => c.refinedOverhang)),
              Math.max(...merged.map((c: Candidate) => c.footprint)),
              Math.max(...merged.map((c: Candidate) => c.maxCross)),
              Math.max(...merged.map((c: Candidate) => c.surfaceQuality)),
              Math.max(...merged.map((c: Candidate) => c.estHeight)),
            ],
          };
          displayResults(merged);
        }
        finishCompute();
        break;
      }
      case 'error': {
        console.error('Worker error:', msg.message);
        finishCompute();
        break;
      }
    }
  };
  worker.onerror = (err) => {
    console.error('Worker error:', err);
    finishCompute();
  };
  worker.postMessage({ data: decimated, config: computeConfig, weights: wArr, ranker: currentRanker, maxCandidates: computeConfig.maxCandidates, minAngleDeg: 15 });
}

function finishCompute(): void {
  progressContainer.style.display = 'none';
  currentWorker = null;
  markClean();

  if (candidates.length === 0) {
    statusEl.textContent = 'No candidates generated';
    return;
  }
  statusEl.textContent = `${candidates.length} candidates — click one to try it`;
  currentIndex = 0;
  viewport.setCriticalAngle(config.criticalAngleDeg);
  viewport.showCandidate(candidates[0].quaternion);
  updateLiveScore(viewport.getMeshQuaternion());
}

function cancelCompute(): void {
  if (currentWorker) { currentWorker.terminate(); currentWorker = null; }
  progressContainer.style.display = 'none';
}

cancelBtn.addEventListener('click', cancelCompute);

function showCandidate(index: number): void {
  if (index < 0 || index >= candidates.length) return;
  currentIndex = index;
  viewport.setCriticalAngle(config.criticalAngleDeg);
  viewport.showCandidate(candidates[index].quaternion);
  updateLiveScore(viewport.getMeshQuaternion());
  displayResults(candidates);
}

const angleSlider = document.getElementById('angle-slider') as HTMLInputElement;
const angleValue = document.getElementById('angle-value')!;
angleSlider.addEventListener('input', () => {
  config.criticalAngleDeg = parseFloat(angleSlider.value);
  angleValue.textContent = angleSlider.value;
  viewport.setCriticalAngle(config.criticalAngleDeg);
  saveConfig();
  markDirty();
  if (liveData) updateLiveScore(viewport.getMeshQuaternion());
});

const hullSphereToggle = document.getElementById('hull-sphere-toggle') as HTMLInputElement;
hullSphereToggle.addEventListener('change', () => {
  config.mode = hullSphereToggle.checked ? 'hull_plus_sphere' : 'hull';
  saveConfig();
  markDirty();
});

const conventionSelect = document.getElementById('convention-select') as HTMLSelectElement;
conventionSelect.addEventListener('change', () => {
  loadConvention = conventionSelect.value as LoadConvention;
  saveConfig();
  markDirty();
});

const PROFILE_LABELS: Record<string, string> = {
  'overhang-only': 'Minimize Supports',
  'footprint-only': 'Minimize Footprint',
  'cross-only': 'Structural Strength',
  'surface-only': 'Best Surface Quality',
  'height-only': 'Fast Print',
  'equal': 'Balanced',
  'resin-biased': 'Resin Printing',
  'overhang-footprint': 'Support + Footprint',
};

const RANKER_LABELS: Record<string, string> = {
  'consensus': 'Consensus',
  'weights': 'Weighted Sum',
  'topsis': 'TOPSIS',
};

const RANKER_HINTS: Record<string, string> = {
  'consensus': 'Minimax: score = 1 − worst normalized cost across all metrics. Bars show per-metric quality vs candidate range.',
  'weights': 'Weighted average of normalized costs using profile weights. Each metric contributes proportionally to its weight.',
  'topsis': 'TOPSIS: ranks by distance to ideal vs anti-ideal solution. Falls back to consensus for live display.',
};

const profileSelect = document.getElementById('profile-select') as HTMLSelectElement;
profileSelect.innerHTML = Object.keys(WEIGHT_PRESETS).map(name =>
  `<option value="${name}" ${name === currentProfile ? 'selected' : ''}>${PROFILE_LABELS[name] ?? name}</option>`
).join('');
profileSelect.addEventListener('change', () => {
  currentProfile = profileSelect.value;
  if (candidates.length > 0 && lastOriData) { spawnCompute(lastOriData); }
  saveConfig();
  markDirty();
  if (liveData) updateLiveScore(viewport.getMeshQuaternion());
});

const rankerSelect = document.getElementById('ranker-select') as HTMLSelectElement;
rankerSelect.addEventListener('change', () => {
  currentRanker = rankerSelect.value;
  saveConfig();
  markDirty();
  if (liveData) updateLiveScore(viewport.getMeshQuaternion());
});

recalcBtn.addEventListener('click', async () => {
  if (!lastFileBytes) return;
  cancelCompute();
  progressContainer.style.display = 'block';
  progressLabel.textContent = 'Reparsing STL...';
  progressBar.className = 'progress-bar-fill indeterminate';
  await paint();
  const data = parseCurrentData();
  if (!data) { statusEl.textContent = 'No data to recalculate'; progressContainer.style.display = 'none'; return; }
  positions = data.positions; faceNormals = data.normals; areas = data.areas; lastOriData = data;
  const decimated = decimateForScore(data, 12000);
  liveData = { positions: decimated.positions, normals: decimated.normals, areas: decimated.areas };
  viewport.setCriticalAngle(config.criticalAngleDeg);
  computeNormBounds(data);
  updateLiveScore(viewport.getMeshQuaternion());
  if (candidates.length > 0) {
    spawnCompute(data);
  } else {
    progressContainer.style.display = 'none';
    markClean();
  }
});

fileInput.addEventListener('change', () => {
  if (fileInput.files && fileInput.files.length > 0) handleFile(fileInput.files[0]);
});
dropZone.addEventListener('dragover', (e) => { e.preventDefault(); dropZone.classList.add('drag-over'); });
dropZone.addEventListener('dragleave', () => { dropZone.classList.remove('drag-over'); });
dropZone.addEventListener('drop', (e) => {
  e.preventDefault(); dropZone.classList.remove('drag-over');
  if (e.dataTransfer?.files && e.dataTransfer.files.length > 0) handleFile(e.dataTransfer.files[0]);
});

findBtn.addEventListener('click', () => {
  if (!lastOriData) return;
  spawnCompute(lastOriData);
});

exportBtn.addEventListener('click', () => {
  if (!positions) return;
  const q = viewport.getMeshQuaternion();
  exportSTL(rotatePositions(positions, q), stlName, currentIndex + 1);
});

function displayResults(cands: Candidate[]): void {
  candidatesSection.style.display = 'block';
  candidateList.innerHTML = cands.map((c, i) =>
    `<li class="${i === currentIndex ? 'active' : ''}" data-index="${i}">#${i + 1} — ${(c.compositeScore * 100).toFixed(0)}%` +
    `<span class="info-icon" title="o: ${c.overhangPenalty.toFixed(0)} f: ${c.footprint.toFixed(0)} x: ${c.maxCross.toFixed(0)} s: ${(c.shadowed * 100).toFixed(0)}% q: ${c.surfaceQuality.toFixed(2)} · H: ${c.estHeight.toFixed(1)}mm · ${c.stability}">ⓘ</span></li>`
  ).join('');
}

candidateList.addEventListener('click', (e) => {
  const li = (e.target as HTMLElement).closest('li');
  if (!li || !li.dataset.index) return;
  showCandidate(parseInt(li.dataset.index));
});

boot();
