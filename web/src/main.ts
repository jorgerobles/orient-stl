import { initWasm, loadSTLBytes, prepareData } from './loadSTL';
import { Viewport } from './viewport';
import { exportSTL } from './exportSTL';
import { rotatePositions } from './rotate';
import { applyConvention } from './convention';
import type { LoadConvention } from './convention';
import { decimateForScore, footprintArea, maxCrossSection, misalignmentScore, computeHeight } from './compute';
import { mergeCandidates, rankByConsensus, rankByWeights, rankByTopsis, WEIGHT_PRESETS } from './compute';
import type { OriData, Candidate, ComputeConfig, SliceResult } from './compute';
import { refine_orientation } from '../pkg/orient_core.js';
import { defaultConfig } from './types';

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

const paint = () => new Promise<void>(r => setTimeout(r, 0));

const statusEl = document.getElementById('status')!;
const fileInput = document.getElementById('file-input') as HTMLInputElement;
const dropZone = document.getElementById('drop-zone')!;
const resultsEl = document.getElementById('results')!;
const viewportContainer = document.getElementById('viewport')!;
const candidateBar = document.getElementById('candidate-bar')!;
const candidateInfoEl = document.getElementById('candidate-info')!;
const progressContainer = document.getElementById('progress-container')!;
const progressBar = document.getElementById('progress-bar')!;
const progressLabel = document.getElementById('progress-label')!;
const cancelBtn = document.getElementById('cancel-btn')!;
const panelLeft = document.getElementById('panel-left')!;
const panelRight = document.getElementById('results')!;

viewport = new Viewport(viewportContainer);

let overlayActive = false;
let overlayKeyHandler: ((e: KeyboardEvent) => void) | null = null;

function enterOverlay(candidateIndex: number): void {
  if (overlayActive || candidates.length === 0) return;
  showCandidate(candidateIndex);
  overlayActive = true;

  panelLeft.style.display = 'none';
  panelRight.style.display = 'none';
  viewportContainer.classList.add('overlay-active');
  candidateBar.classList.add('visible');

  overlayKeyHandler = (e: KeyboardEvent) => {
    if (e.key === 'Escape') exitOverlay();
    if (e.key === 'ArrowLeft' && currentIndex > 0) showCandidate(currentIndex - 1);
    if (e.key === 'ArrowRight' && currentIndex < candidates.length - 1) showCandidate(currentIndex + 1);
  };
  document.addEventListener('keydown', overlayKeyHandler);

  viewport.enterOverlayMode(updateLiveScore);
}

function exitOverlay(): void {
  if (!overlayActive) return;
  overlayActive = false;
  viewport.exitOverlayMode();

  viewportContainer.classList.remove('overlay-active');
  candidateBar.classList.remove('visible');
  panelLeft.style.display = '';
  panelRight.style.display = 'block';

  if (overlayKeyHandler) {
    document.removeEventListener('keydown', overlayKeyHandler);
    overlayKeyHandler = null;
  }
}

function applyQuat(q: [number, number, number, number], v: [number, number, number]): [number, number, number] {
  const [w, x, y, z] = q;
  const [vx, vy, vz] = v;
  const uv_x = y * vz - z * vy;
  const uv_y = z * vx - x * vz;
  const uv_z = x * vy - y * vx;
  const uuv_x = y * uv_z - z * uv_y;
  const uuv_y = z * uv_x - x * uv_z;
  const uuv_z = x * uv_y - y * uv_x;
  return [vx + 2 * (w * uv_x + uuv_x), vy + 2 * (w * uv_y + uuv_y), vz + 2 * (w * uv_z + uuv_z)];
}

function updateLiveScore(q: [number, number, number, number]): void {
  if (!liveData || candidates.length === 0) return;
  const invQ: [number, number, number, number] = [q[0], -q[1], -q[2], -q[3]];
  const dir = applyQuat(invQ, [0, -1, 0]);

  // WASM = single source of truth for overhang.
  const { positions: lp, normals: ln, areas: la } = liveData;
  let refined: number;
  try {
    const res = refine_orientation(lp, ln, la, dir[0], dir[1], dir[2], config.criticalAngleDeg, config.refineIterations ?? 0, 42);
    refined = res[3];
  } catch {
    refined = 0;
  }

  const foot = footprintArea(dir, ln, la);
  const cross = maxCrossSection(dir, lp, ln, la, 64);
  const surf = misalignmentScore(dir, ln, la);
  const height = computeHeight(dir, lp);

  // Fixed normalization range from candidates only (stable as you drag).
  // Weighted average with profile weights — no single metric can kill the score.
  const oVals = candidates.map(c => c.refinedOverhang);
  const fVals = candidates.map(c => c.footprint);
  const cVals = candidates.map(c => c.maxCross);
  const sVals = candidates.map(c => c.surfaceQuality);
  const hVals = candidates.map(c => c.estHeight);
  const oL = Math.min(...oVals), oH = Math.max(...oVals);
  const fL = Math.min(...fVals), fH = Math.max(...fVals);
  const cL = Math.min(...cVals), cH = Math.max(...cVals);
  const sL = Math.min(...sVals), sH = Math.max(...sVals);
  const hL = Math.min(...hVals), hH = Math.max(...hVals);
  const oS = Math.max(oH - oL, 1e-9), fS = Math.max(fH - fL, 1e-9);
  const cS = Math.max(cH - cL, 1e-9), sS = Math.max(sH - sL, 1e-9);
  const hS = Math.max(hH - hL, 1e-9);
  const clamp = (v: number) => v < 0 ? 0 : v > 1 ? 1 : v;

  const w = WEIGHT_PRESETS[currentProfile] ?? WEIGHT_PRESETS['resin-biased'];
  const wSum = w.wOverhang + w.wFootprint + w.wCross + w.wSurface + w.wHeight;
  const score = wSum > 0 ? 1 - (
    w.wOverhang * clamp((refined - oL) / oS) +
    w.wFootprint * clamp((foot - fL) / fS) +
    w.wCross * clamp((cross - cL) / cS) +
    w.wSurface * clamp((surf - sL) / sS) +
    w.wHeight * clamp((height - hL) / hS)
  ) / wSum : 1 - clamp((refined - oL) / oS);

  candidateInfoEl.innerHTML = `<span class="score">${(score * 100).toFixed(0)}%</span>`;
}

document.getElementById('exit-btn')!.addEventListener('click', exitOverlay);
document.getElementById('reset-btn')!.addEventListener('click', () => {
  if (candidates.length === 0) return;
  viewport.showCandidate(candidates[currentIndex].quaternion);
  updateCandidateBar();
});
document.getElementById('prev-btn')!.addEventListener('click', () => { if (currentIndex > 0) showCandidate(currentIndex - 1); });
document.getElementById('next-btn')!.addEventListener('click', () => { if (currentIndex < candidates.length - 1) showCandidate(currentIndex + 1); });

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
  if (overlayActive) exitOverlay();
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

    spawnCompute(fullData);
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
  panelRight.style.display = 'none';
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
      viewport.setCriticalAngle(config.criticalAngleDeg);
      viewport.showCandidate(merged[0].quaternion);
      panelRight.style.display = 'block';
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
  statusEl.textContent = `${candidates.length} candidates — click one to inspect`;
  currentIndex = 0;
  viewport.setCriticalAngle(config.criticalAngleDeg);
  viewport.showCandidate(candidates[0].quaternion);
  panelRight.style.display = 'block';
  displayResults(candidates);
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
  updateCandidateBar();
  displayResults(candidates);
}

function updateCandidateBar(): void {
  const c = candidates[currentIndex];
  const prevBtn = document.getElementById('prev-btn') as HTMLButtonElement;
  const nextBtn = document.getElementById('next-btn') as HTMLButtonElement;
  prevBtn.disabled = currentIndex <= 0;
  nextBtn.disabled = currentIndex >= candidates.length - 1;
  candidateInfoEl.innerHTML =
    `<span style="color:#888;font-size:0.8rem">#${currentIndex + 1}/${candidates.length}</span>` +
    `<span class="score">${(c.compositeScore * 100).toFixed(0)}%</span>` +
    `${currentIndex === 0 ? '<span class="best">★ BEST</span>' : ''}`;
}

const angleSlider = document.getElementById('angle-slider') as HTMLInputElement;
const angleValue = document.getElementById('angle-value')!;
angleSlider.addEventListener('input', () => {
  config.criticalAngleDeg = parseFloat(angleSlider.value);
  angleValue.textContent = angleSlider.value;
  viewport.setCriticalAngle(config.criticalAngleDeg);
  saveConfig();
  markDirty();
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

const profileSelect = document.getElementById('profile-select') as HTMLSelectElement;
profileSelect.innerHTML = Object.keys(WEIGHT_PRESETS).map(name =>
  `<option value="${name}" ${name === currentProfile ? 'selected' : ''}>${PROFILE_LABELS[name] ?? name}</option>`
).join('');
profileSelect.addEventListener('change', () => {
  currentProfile = profileSelect.value;
  if (candidates.length > 0) { candidates = applyCurrentRank(candidates); displayResults(candidates); }
  saveConfig();
  markDirty();
});

const rankerSelect = document.getElementById('ranker-select') as HTMLSelectElement;
rankerSelect.addEventListener('change', () => {
  currentRanker = rankerSelect.value;
  saveConfig();
  markDirty();
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
  spawnCompute(data);
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

document.getElementById('export-btn')!.addEventListener('click', () => {
  if (!positions || candidates.length === 0) return;
  const c = candidates[currentIndex];
  const quat: [number, number, number, number] = [c.quaternion[0], c.quaternion[1], c.quaternion[2], c.quaternion[3]];
  const qres = quat;
  exportSTL(rotatePositions(positions, qres), stlName, currentIndex + 1);
});

function displayResults(cands: Candidate[]): void {
  resultsEl.innerHTML = `<h3>Candidates (${cands.length})</h3><ol>${cands.map((c, i) =>
    `<li class="${i === currentIndex ? 'active' : ''}" data-index="${i}">#${i + 1} — ${(c.compositeScore * 100).toFixed(0)}%` +
    `<span class="info-icon" title="o: ${c.overhangPenalty.toFixed(0)} f: ${c.footprint.toFixed(0)} x: ${c.maxCross.toFixed(0)} s: ${(c.shadowed * 100).toFixed(0)}% q: ${c.surfaceQuality.toFixed(2)} · H: ${c.estHeight.toFixed(1)}mm · ${c.stability}">ⓘ</span></li>`
  ).join('')}</ol>`;
}

resultsEl.addEventListener('click', (e) => {
  const li = (e.target as HTMLElement).closest('li');
  if (!li || !li.dataset.index) return;
  enterOverlay(parseInt(li.dataset.index));
});

boot();
