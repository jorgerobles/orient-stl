import { initWasm, loadSTLBytes, prepareData } from './loadSTL';
import { Viewport } from './viewport';
import { exportSTL } from './exportSTL';
import { decimateForScore } from './compute';
import { mergeCandidates, rankByConsensus } from './compute';
import type { OriData, Candidate, ComputeConfig, SliceResult } from './compute';
import { defaultConfig } from './types';

let config = defaultConfig();
let candidates: Candidate[] = [];
let positions: Float32Array | null = null;
let faceNormals: Float32Array | null = null;
let currentIndex = 0;
let yawOffset = 0;
let stlName = '';
let viewport: Viewport;
let workers: Worker[] = [];

const paint = () => new Promise<void>(r => setTimeout(r, 0));

const statusEl = document.getElementById('status')!;
const fileInput = document.getElementById('file-input') as HTMLInputElement;
const dropZone = document.getElementById('drop-zone')!;
const resultsEl = document.getElementById('results')!;
const viewportContainer = document.getElementById('viewport')!;
const navEl = document.getElementById('nav-controls')!;
const yawPanel = document.getElementById('yaw-panel')!;
const exportBtn = document.getElementById('export-btn') as HTMLButtonElement;
const progressContainer = document.getElementById('progress-container')!;
const progressBar = document.getElementById('progress-bar')!;
const progressLabel = document.getElementById('progress-label')!;
const cancelBtn = document.getElementById('cancel-btn')!;

viewport = new Viewport(viewportContainer);

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
  cancelCompute();
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

    await paint();

    progressLabel.textContent = 'Parsing STL (WASM)...';
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

    await paint();

    progressLabel.textContent = 'Rendering model...';
    statusEl.textContent = 'Rendering model...';
    viewport.loadModel(positions, faceNormals);
    viewport.resetCamera();
    navEl.style.display = 'flex';
    document.getElementById('candidate-info')!.textContent = 'Computing candidates...';

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
  yawPanel.style.display = 'none';
  exportBtn.style.display = 'none';
  resultsEl.innerHTML = '';
  candidates = [];

  const computeConfig: ComputeConfig = {
    criticalAngleDeg: config.criticalAngleDeg,
    excludeUnstable: config.excludeUnstable,
    maxCandidates: config.maxCandidates,
  };

  await paint();
  const decimated = decimateForScore(data, 12000);
  progressLabel.textContent = 'Computing candidates...';

  // Build segmented progress bar
  const numDirs = decimated.directions.length / 3;
  const numWorkers = workerCount();
  const dirsPerWorker = Math.ceil(numDirs / numWorkers);
  progressBar.className = 'progress-segments';
  progressBar.style.width = '';  // clear HTML inline width: 0% so CSS width:100% takes effect
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
      yawOffset = 0;
      viewport.setCriticalAngle(config.criticalAngleDeg);
      viewport.showCandidate(merged[0].quaternion);
      navEl.style.display = 'flex';
      yawPanel.style.display = 'block';
      exportBtn.style.display = 'inline-block';
      updateNavInfo();
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
          if (completedWorkers >= numWorkers) {
            finishCompute();
          }
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

  const computeConfig: ComputeConfig = {
    criticalAngleDeg: config.criticalAngleDeg,
    excludeUnstable: config.excludeUnstable,
    maxCandidates: config.maxCandidates,
  };

  // Read allSlices via stored candidates — already set by mergeAndShow
  if (candidates.length === 0) {
    statusEl.textContent = 'No candidates generated';
    return;
  }
  candidates = applyCurrentRank(candidates);
  statusEl.textContent = `${candidates.length} candidates found`;
  currentIndex = 0;
  yawOffset = 0;
  viewport.setCriticalAngle(config.criticalAngleDeg);
  viewport.showCandidate(candidates[0].quaternion);
  navEl.style.display = 'flex';
  yawPanel.style.display = 'block';
  exportBtn.style.display = 'inline-block';
  updateNavInfo();
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
  yawOffset = 0;
  viewport.setCriticalAngle(config.criticalAngleDeg);
  viewport.showCandidate(candidates[index].quaternion);
  updateYawDisplay();
  updateNavInfo();
  displayResults(candidates);
}

function updateNavInfo(): void {
  const c = candidates[currentIndex];
  document.getElementById('candidate-info')!.innerHTML =
    `<strong>#${currentIndex + 1}/${candidates.length}</strong> — ${(c.compositeScore * 100).toFixed(0)}%` +
    `${currentIndex === 0 ? ' ★ BEST' : ''}`;
}

function updateYawDisplay(): void {
  const slider = document.getElementById('yaw-slider') as HTMLInputElement;
  const val = document.getElementById('yaw-value')!;
  slider.value = '0';
  val.textContent = '0°';
}

const angleSlider = document.getElementById('angle-slider') as HTMLInputElement;
const angleValue = document.getElementById('angle-value')!;
angleSlider.addEventListener('input', () => {
  config.criticalAngleDeg = parseFloat(angleSlider.value);
  angleValue.textContent = angleSlider.value;
  viewport.setCriticalAngle(config.criticalAngleDeg);
});

function rerank(): void {
  if (candidates.length === 0) return;
  candidates = rankByConsensus(candidates);
  currentIndex = 0;
  yawOffset = 0;
  viewport.showCandidate(candidates[0].quaternion);
  updateNavInfo();
  displayResults(candidates);
}

fileInput.addEventListener('change', () => {
  if (fileInput.files && fileInput.files.length > 0) handleFile(fileInput.files[0]);
});
dropZone.addEventListener('dragover', (e) => { e.preventDefault(); dropZone.classList.add('drag-over'); });
dropZone.addEventListener('dragleave', () => { dropZone.classList.remove('drag-over'); });
dropZone.addEventListener('drop', (e) => {
  e.preventDefault(); dropZone.classList.remove('drag-over');
  if (e.dataTransfer?.files && e.dataTransfer.files.length > 0) handleFile(e.dataTransfer.files[0]);
});

document.getElementById('prev-btn')!.addEventListener('click', () => { if (currentIndex > 0) showCandidate(currentIndex - 1); });
document.getElementById('next-btn')!.addEventListener('click', () => { if (currentIndex < candidates.length - 1) showCandidate(currentIndex + 1); });

const yawSlider = document.getElementById('yaw-slider') as HTMLInputElement;
const yawValue = document.getElementById('yaw-value')!;
yawSlider.addEventListener('input', () => {
  yawValue.textContent = `${parseFloat(yawSlider.value)}°`;
});
yawSlider.addEventListener('change', () => {
  yawOffset = parseFloat(yawSlider.value);
  viewport.applyYaw(yawOffset);
});
document.getElementById('yaw-reset-btn')!.addEventListener('click', () => {
  yawOffset = 0; yawSlider.value = '0'; yawValue.textContent = '0°';
  viewport.showCandidate(candidates[currentIndex].quaternion);
});
document.getElementById('snap-btn')!.addEventListener('click', () => {
  const snaps = [0, 45, 90, 135, 180, 225, 270, 315];
  let closest = snaps[0], minDiff = Math.abs(yawOffset - closest);
  for (const a of snaps) { const d = Math.abs(yawOffset - a); if (d < minDiff) { minDiff = d; closest = a; } }
  yawOffset = closest;
  (document.getElementById('yaw-slider') as HTMLInputElement).value = String(closest);
  yawValue.textContent = `${closest}°`;
  viewport.applyYaw(closest);
});

exportBtn.addEventListener('click', () => {
  if (!positions || candidates.length === 0) return;
  const c = candidates[currentIndex];
  const quat: [number, number, number, number] = [c.quaternion[0], c.quaternion[1], c.quaternion[2], c.quaternion[3]];
  const yawRad = (yawOffset * Math.PI) / 180;
  const qyaw: [number, number, number, number] = [Math.cos(yawRad / 2), 0, Math.sin(yawRad / 2), 0];
  const qres = multiplyQuats(qyaw, quat);
  exportSTL(rotatePositions(positions, qres), stlName, currentIndex + 1);
});

function multiplyQuats(a: [number, number, number, number], b: [number, number, number, number]): [number, number, number, number] {
  return [
    a[0] * b[0] - a[1] * b[1] - a[2] * b[2] - a[3] * b[3],
    a[0] * b[1] + a[1] * b[0] + a[2] * b[3] - a[3] * b[2],
    a[0] * b[2] - a[1] * b[3] + a[2] * b[0] + a[3] * b[1],
    a[0] * b[3] + a[1] * b[2] - a[2] * b[1] + a[3] * b[0],
  ];
}

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
  resultsEl.innerHTML = `<h3>Candidates (${cands.length}) — #1 = best ↓</h3><ol>${cands.map((c, i) =>
    `<li class="${i === currentIndex ? 'active' : ''}" data-index="${i}">#${i + 1} — ${(c.compositeScore * 100).toFixed(0)}%` +
    `<span class="info-icon" title="o: ${c.overhangPenalty.toFixed(0)} f: ${c.footprint.toFixed(0)} x: ${c.maxCross.toFixed(0)} s: ${(c.shadowed * 100).toFixed(0)}% · H: ${c.estHeight.toFixed(1)}mm · ${c.stability}">ⓘ</span></li>`
  ).join('')}</ol>`;
}

// Click on candidate list items to view them
resultsEl.addEventListener('click', (e) => {
  const li = (e.target as HTMLElement).closest('li');
  if (!li || !li.dataset.index) return;
  showCandidate(parseInt(li.dataset.index));
});

boot();
