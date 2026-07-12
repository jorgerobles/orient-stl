import { initWasm, loadSTLBytes, prepareData } from './loadSTL';
import { Viewport } from './viewport';
import { exportSTL } from './exportSTL';
import { decimateForScore, nearestCandidateScore } from './compute';
import { mergeCandidates, rankByConsensus } from './compute';
import type { OriData, Candidate, ComputeConfig, SliceResult } from './compute';
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

function updateLiveScore(q: [number, number, number, number]): void {
  const nearest = nearestCandidateScore(q, candidates);
  candidateInfoEl.innerHTML =
    `<span class="score">${(nearest.score * 100).toFixed(0)}%</span>`;
}

document.getElementById('varita-btn')!.addEventListener('click', async () => {
  if (!positions || !faceNormals || !areas || candidates.length === 0 || !overlayActive) return;

  const varitaBtn = document.getElementById('varita-btn') as HTMLButtonElement;
  varitaBtn.disabled = true;
  varitaBtn.textContent = 'Refining...';
  candidateInfoEl.innerHTML = `<span class="score" style="font-size:0.9rem;color:#888">Refining...</span>`;

  try {
    const q = viewport.getMeshQuaternion();
    const invQ: [number, number, number, number] = [q[0], -q[1], -q[2], -q[3]];
    const dir = (() => {
      const [w, x, y, z] = invQ;
      const v: [number, number, number] = [0, -1, 0];
      const uv_x = y * v[2] - z * v[1];
      const uv_y = z * v[0] - x * v[2];
      const uv_z = x * v[1] - y * v[0];
      const uuv_x = y * uv_z - z * uv_y;
      const uuv_y = z * uv_x - x * uv_z;
      const uuv_z = x * uv_y - y * uv_x;
      return [
        v[0] + 2.0 * (w * uv_x + uuv_x),
        v[1] + 2.0 * (w * uv_y + uuv_y),
        v[2] + 2.0 * (w * uv_z + uuv_z),
      ] as [number, number, number];
    })();

    const wasmModule = await import('../pkg/orient_core.js');
    const result = (wasmModule as any).refine_orientation(
      positions, faceNormals, areas,
      dir[0], dir[1], dir[2],
      config.criticalAngleDeg, 50,
    );

    const refinedDir: [number, number, number] = [result[0], result[1], result[2]];
    const THREE = await import('three');
    const newQuat = new THREE.Quaternion().setFromUnitVectors(
      new THREE.Vector3(refinedDir[0], refinedDir[1], refinedDir[2]),
      new THREE.Vector3(0, -1, 0),
    );
    viewport.showCandidate([newQuat.x, newQuat.y, newQuat.z, newQuat.w]);
    updateLiveScore([newQuat.x, newQuat.y, newQuat.z, newQuat.w]);
  } catch (err) {
    console.error('Hill-climb failed:', err);
    candidateInfoEl.innerHTML = `<span class="score" style="color:#e74c3c">Error</span>`;
  }

  varitaBtn.textContent = '✨ Varita';
  varitaBtn.disabled = false;
});

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
    stlName = file.name;

    progressLabel.textContent = 'Parsing STL (WASM)...';
    await paint();
    const raw = prepareData(bytes, config as unknown as Record<string, unknown>) as unknown as {
      positions: number[]; normals: number[]; areas: number[]; directions: number[];
    };

    if (raw.positions.length === 0) throw new Error('No triangles in STL');
    if (raw.directions.length === 0) throw new Error('No candidates generated');

    const fullData: OriData = {
      positions: new Float32Array(raw.positions),
      normals: new Float32Array(raw.normals),
      areas: new Float32Array(raw.areas),
      directions: new Float32Array(raw.directions),
    };

    positions = fullData.positions;
    faceNormals = fullData.normals;
    areas = fullData.areas;

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

function applyCurrentRank(raw: Candidate[]): Candidate[] {
  return rankByConsensus(raw);
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
  };

  await paint();
  const decimated = decimateForScore(data, 12000);
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
    const merged = applyCurrentRank(mergeCandidates(allSlices, computeConfig));
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
});

const hullSphereToggle = document.getElementById('hull-sphere-toggle') as HTMLInputElement;
hullSphereToggle.addEventListener('change', () => {
  config.mode = hullSphereToggle.checked ? 'hull_plus_sphere' : 'hull';
  if (lastFile) handleFile(lastFile);
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

function rotatePositions(positions: Float32Array, q: [number, number, number, number]): Float32Array {
  const out = new Float32Array(positions.length);
  for (let i = 0; i < positions.length; i += 3) {
    const x = positions[i], y = positions[i + 1], z = positions[i + 2];
    const qx = q[1] * z - q[2] * y, qy = q[2] * x - q[0] * z, qz = q[0] * y - q[1] * x, qw = -q[1] * x - q[2] * y - q[3] * z;
    out[i] = x + 2 * (qw * q[1] + qx * q[0] - qy * q[3] + qz * q[2]);
    out[i + 1] = y + 2 * (qw * q[2] + qy * q[0] - qz * q[1] + qx * q[3]);
    out[i + 2] = z + 2 * (qw * q[3] + qz * q[0] - qx * q[2] + qy * q[1]);
  }
  return out;
}

function displayResults(cands: Candidate[]): void {
  resultsEl.innerHTML = `<h3>Candidates (${cands.length})</h3><ol>${cands.map((c, i) =>
    `<li class="${i === currentIndex ? 'active' : ''}" data-index="${i}">#${i + 1} — ${(c.compositeScore * 100).toFixed(0)}%` +
    `<span class="info-icon" title="o: ${c.overhangPenalty.toFixed(0)} f: ${c.footprint.toFixed(0)} x: ${c.maxCross.toFixed(0)} s: ${(c.shadowed * 100).toFixed(0)}% · H: ${c.estHeight.toFixed(1)}mm · ${c.stability}">ⓘ</span></li>`
  ).join('')}</ol>`;
}

resultsEl.addEventListener('click', (e) => {
  const li = (e.target as HTMLElement).closest('li');
  if (!li || !li.dataset.index) return;
  enterOverlay(parseInt(li.dataset.index));
});

boot();
