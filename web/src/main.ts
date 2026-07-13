import { initWasm, loadSTLBytes, prepareData } from './loadSTL';
import { Viewport } from './viewport';
import { exportSTL } from './exportSTL';
import { rotatePositions } from './rotate';
import { applyConvention } from './convention';
import type { LoadConvention } from './convention';
import { decimateForScore } from './compute';
import { mergeCandidates, rankByConsensus, rankByWeights, rankByTopsis, WEIGHT_PRESETS } from './compute';
import type { OriData, Candidate, ComputeConfig, SliceResult } from './compute';
import { score_orientation } from '../pkg/orient_core.js';
import { defaultConfig } from './types';
import { dirFromQuat } from './quaternion';

let config = defaultConfig();
let candidates: Candidate[] = [];
let positions: Float32Array | null = null;
let faceNormals: Float32Array | null = null;
let areas: Float32Array | null = null;
let currentIndex = 0;
let stlName = '';
let viewport: Viewport;
let workers: Worker[] = [];
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
 *  bounds for live scoring. Cheaper than full candidate search — gives the user
 *  immediate feedback on load. */
function computeNormBounds(data: OriData): void {
  const numDirs = data.directions.length / 3;
  if (numDirs === 0 || !liveData) return;
  const step = Math.max(1, Math.floor(numDirs / 30));
  const vals: number[][] = [[], [], [], [], []];
  const { positions: p, normals: n, areas: a } = liveData;
  for (let i = 0; i < numDirs; i += step) {
    try {
      const raw = score_orientation(p, n, a,
        data.directions[i * 3], data.directions[i * 3 + 1], data.directions[i * 3 + 2],
        config.criticalAngleDeg, 0, 42);
      for (let m = 0; m < 5; m++) vals[m].push(raw[3 + m]);
    } catch { /* skip bad direction */ }
  }
  if (vals[0].length === 0) return;
  normBounds = {
    lo: vals.map(v => Math.min(...v)),
    hi: vals.map(v => Math.max(...v)),
  };
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

  // Overall score: use the SAME ranker as candidates when available.
  // Temporarily add the live direction to the candidate set, run the ranker,
  // and extract its score — guaranteeing identical formula + normalization.
  const w = WEIGHT_PRESETS[currentProfile] ?? WEIGHT_PRESETS['resin-biased'];
  const weights = [w.wOverhang, w.wFootprint, w.wCross, w.wSurface, w.wHeight];
  let score: number;
  if (candidates.length > 0) {
    const liveCand: Candidate = {
      id: '__live__',
      quaternion: [q[0], q[1], q[2], q[3]], // [x, y, z, w]
      overhangPenalty: overhang,
      footprint: foot,
      maxCross: cross,
      shadowed: 0,
      surfaceQuality: surf,
      estHeight: height,
      refinedOverhang: overhang,
      refineVariance: 0,
      stability: 'stable' as const,
      stabilityMargin: 1,
      contactArea: 0,
      compositeScore: 0,
    };
    const ranked = applyCurrentRank([...candidates, liveCand]);
    score = ranked.find(c => c.id === '__live__')?.compositeScore ?? 0;
    // rankByWeights sorts ascending (lower = better), so invert.
    if (currentRanker === 'weights') score = 1 - score;
  } else {
    // No candidates: weighted average from profile weights.
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

function applyCurrentRank(raw: Candidate[]): Candidate[] {
  const weights = WEIGHT_PRESETS[currentProfile] ?? WEIGHT_PRESETS['resin-biased'];
  switch (currentRanker) {
    case 'weights': return rankByWeights(raw, weights);
    case 'topsis': return rankByTopsis(raw, weights);
    case 'consensus':
    default: return rankByConsensus(raw);
  }
}

function workerCount(): number {
  const max = navigator.hardwareConcurrency || 4;
  return Math.max(1, Math.min(max - 1, 6));
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

  const numDirs = decimated.directions.length / 3;
  const numWorkers = workerCount();
  const dirsPerWorker = Math.ceil(numDirs / numWorkers);
  progressBar.className = 'progress-segments';
  progressBar.style.width = '';
  progressBar.innerHTML = '';
  const segFills: HTMLDivElement[] = [];
  for (let i = 0; i < numWorkers; i++) {
    const seg = document.createElement('div');
    seg.className = 'segment';
    seg.style.width = `${100 / numWorkers}%`;
    const fill = document.createElement('div');
    fill.className = 'segment-fill';
    seg.appendChild(fill);
    progressBar.appendChild(seg);
    segFills.push(fill);
  }

  const allSlices: SliceResult[][] = Array.from({ length: numWorkers }, () => []);
  let completedWorkers = 0;

  function mergeAndShow() {
    const weights = WEIGHT_PRESETS[currentProfile] ?? WEIGHT_PRESETS['resin-biased'];
    const merged = applyCurrentRank(mergeCandidates(allSlices, computeConfig, weights, currentRanker));
    if (merged.length > 0) {
      candidates = merged;
      currentIndex = 0;
      // Update normalization bounds from full candidate set.
      normBounds = {
        lo: [
          Math.min(...merged.map(c => c.refinedOverhang)),
          Math.min(...merged.map(c => c.footprint)),
          Math.min(...merged.map(c => c.maxCross)),
          Math.min(...merged.map(c => c.surfaceQuality)),
          Math.min(...merged.map(c => c.estHeight)),
        ],
        hi: [
          Math.max(...merged.map(c => c.refinedOverhang)),
          Math.max(...merged.map(c => c.footprint)),
          Math.max(...merged.map(c => c.maxCross)),
          Math.max(...merged.map(c => c.surfaceQuality)),
          Math.max(...merged.map(c => c.estHeight)),
        ],
      };
      displayResults(merged);
    }
  }

  for (let w = 0; w < numWorkers; w++) {
    const dirStart = w * dirsPerWorker;
    const dirCount = Math.min(dirsPerWorker, numDirs - dirStart);
    if (dirCount <= 0) { completedWorkers++; continue; }

    const worker = new Worker(new URL('./orient.worker.ts', import.meta.url), { type: 'module' });
    workers.push(worker);

    const workerIdx = w;
    worker.onmessage = (e) => {
      const msg = e.data;
      switch (msg.type) {
        case 'progress':
          segFills[workerIdx].style.width = `${msg.value}%`;
          break;
        case 'slice-done': {
          allSlices[workerIdx] = msg.results as SliceResult[];
          completedWorkers++;
          segFills[workerIdx].style.width = '100%';
          progressBar.children[workerIdx]?.classList.add('done');
          mergeAndShow();
          if (completedWorkers >= numWorkers) finishCompute();
          break;
        }
      }
    };
    worker.onerror = (err) => {
      console.error(`Worker ${workerIdx} error:`, err);
      completedWorkers++;
      progressBar.children[workerIdx]?.classList.add('done');
      segFills[workerIdx].style.width = '100%';
      if (completedWorkers >= numWorkers) finishCompute();
    };
    worker.postMessage({ data: decimated, config: computeConfig, dirStart, dirCount });
  }
}

function finishCompute(): void {
  progressContainer.style.display = 'none';
  workers = [];
  markClean();

  if (candidates.length === 0) {
    statusEl.textContent = 'No candidates generated';
    return;
  }
  candidates = applyCurrentRank(candidates);
  statusEl.textContent = `${candidates.length} candidates — click one to try it`;
  currentIndex = 0;
  viewport.setCriticalAngle(config.criticalAngleDeg);
  viewport.showCandidate(candidates[0].quaternion);
  updateLiveScore(viewport.getMeshQuaternion());
}

function cancelCompute(): void {
  for (const w of workers) { w.terminate(); }
  workers = [];
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
  if (candidates.length > 0) { candidates = applyCurrentRank(candidates); displayResults(candidates); }
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
