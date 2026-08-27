import type { OriData, Candidate, ComputeConfig, SupportConfig, SupportResult } from './types';

// ─── Worker message types ───

interface ParseRequest { type: 'parse'; bytes: Uint8Array; }
interface RepairRequest { type: 'repair'; positions: Float32Array; weldEpsilon: number; maxHoleEdges: number; }
interface MeshRequest { type: 'mesh'; positions: Float32Array; }
interface ScoreRequest {
  type: 'score';
  data: OriData;
  config: ComputeConfig;
  weights: [number, number, number, number, number, number];
  ranker: string;
  maxCandidates: number;
  minAngleDeg: number;
  normLo: number[] | null;
  normHi: number[] | null;
}

interface SupportRequest {
  type: 'support';
  positions: Float32Array;
  normals: Float32Array;
  areas: Float32Array;
  direction: Float32Array;
  config: SupportConfig;
}

type WorkerRequest = ParseRequest | RepairRequest | MeshRequest | ScoreRequest | SupportRequest;

interface ResultResponse { type: 'result'; positions: Float32Array; }
interface MeshResponse { type: 'result'; positions: Float32Array; normals: Float32Array; areas: Float32Array; }
interface ScoreResponse { type: 'results'; candidates: Candidate[]; }
interface SupportResponse { type: 'result'; supports: SupportResult; }
interface ProgressResponse { type: 'progress'; value: number; }
interface ErrorResponse { type: 'error'; message: string; }

type WorkerResponse = ResultResponse | MeshResponse | ScoreResponse | SupportResponse | ProgressResponse | ErrorResponse;

// ─── Pipeline types ───

export interface PipelineConfig {
  autoRepair: boolean;
  weldEpsilon: number;
  maxHoleEdges: number;
  orient: ComputeConfig;
  weights: [number, number, number, number, number, number];
  ranker: string;
  maxCandidates: number;
  minAngleDeg: number;
  normLo?: number[];
  normHi?: number[];
  generateSupports?: boolean;
  support?: SupportConfig;
}

export interface PipelineResult {
  positions: Float32Array;
  normals: Float32Array;
  areas: Float32Array;
  candidates: Candidate[];
  supports?: SupportResult;
}

export type ProgressCallback = (stage: string, pct: number) => void;

// ─── Worker helpers ───

function runWorker<T extends WorkerResponse>(
  worker: Worker,
  message: WorkerRequest,
  transfer?: Transferable[],
): Promise<T> {
  return new Promise((resolve, reject) => {
    const handler = (e: MessageEvent<T>) => {
      worker.removeEventListener('message', handler);
      worker.removeEventListener('error', errorHandler);
      resolve(e.data);
    };
    const errorHandler = (e: ErrorEvent) => {
      worker.removeEventListener('message', handler);
      worker.removeEventListener('error', errorHandler);
      reject(new Error(e.message || 'Worker error'));
    };
    worker.addEventListener('message', handler);
    worker.addEventListener('error', errorHandler);
    worker.postMessage(message, transfer ?? []);
  });
}

// ─── Pipeline ───

export async function runPipeline(
  bytes: Uint8Array,
  config: PipelineConfig,
  onProgress: ProgressCallback,
): Promise<PipelineResult> {
  const yieldToBrowser = () => new Promise<void>(r => setTimeout(r, 0));

  // Stage 1: Parse STL
  onProgress('Parsing STL...', 5);
  const parseWorker = new Worker(
    new URL('./workers/stl-parse.worker.ts', import.meta.url),
    { type: 'module' },
  );
  const parsed = await runWorker<ResultResponse>(parseWorker, { type: 'parse', bytes });
  parseWorker.terminate();
  await yieldToBrowser();

  if (!parsed.positions || parsed.positions.length === 0) {
    throw new Error('No triangles found in STL');
  }

  let positions = parsed.positions;

  // Stage 2: Repair (optional)
  if (config.autoRepair) {
    onProgress('Repairing mesh...', 30);
    const repairWorker = new Worker(
      new URL('./workers/stl-repair.worker.ts', import.meta.url),
      { type: 'module' },
    );
    const repaired = await runWorker<ResultResponse>(repairWorker, {
      type: 'repair',
      positions,
      weldEpsilon: config.weldEpsilon,
      maxHoleEdges: config.maxHoleEdges,
    });
    repairWorker.terminate();
    positions = repaired.positions;
    await yieldToBrowser();
  }

  // Stage 3: Mesh precomputation
  onProgress('Precomputing mesh...', 60);
  const meshWorker = new Worker(
    new URL('./workers/mesher.worker.ts', import.meta.url),
    { type: 'module' },
  );
  const meshed = await runWorker<MeshResponse>(meshWorker, { type: 'mesh', positions });
  meshWorker.terminate();
  await yieldToBrowser();

  // Stage 4: Orient scoring
  onProgress('Scoring orientations...', 80);
  const orientWorker = new Worker(
    new URL('./workers/orient.worker.ts', import.meta.url),
    { type: 'module' },
  );

  const data: OriData = {
    positions: meshed.positions,
    normals: meshed.normals,
    areas: meshed.areas,
    directions: new Float32Array(0), // placeholder — orient worker will compute from hull
  };

  // Forward orient worker progress to pipeline callback
  const orientHandler = (e: MessageEvent<WorkerResponse>) => {
    if (e.data.type === 'progress') {
      onProgress('Scoring orientations...', 80 + Math.round((e.data as ProgressResponse).value * 0.19));
    }
  };
  orientWorker.addEventListener('message', orientHandler);

  const scoreResult = await runWorker<ScoreResponse>(orientWorker, {
    type: 'score',
    data,
    config: config.orient,
    weights: config.weights,
    ranker: config.ranker,
    maxCandidates: config.maxCandidates,
    minAngleDeg: config.minAngleDeg,
    normLo: config.normLo ?? null,
    normHi: config.normHi ?? null,
  });

  orientWorker.removeEventListener('message', orientHandler);
  orientWorker.terminate();
  onProgress('Scoring orientations...', 90);

  // Stage 5: Support generation (optional)
  let supports: SupportResult | undefined;
  if (config.generateSupports && scoreResult.candidates.length > 0) {
    onProgress('Generating supports...', 92);
    const supportWorker = new Worker(
      new URL('./workers/support.worker.ts', import.meta.url),
      { type: 'module' },
    );

    // Extract direction from best candidate quaternion
    const bestQ = scoreResult.candidates[0].quaternion;
    // Quaternion to direction: apply q to [0, -1, 0] (build direction)
    const qw = bestQ[0], qx = bestQ[1], qy = bestQ[2], qz = bestQ[3];
    // Rotate [0, -1, 0] by quaternion
    const dx = 2 * (qx * qz + qw * qy);
    const dy = 2 * (qy * qz - qw * qx);
    const dz = 1 - 2 * (qx * qx + qy * qy);
    const direction = new Float32Array([dx, dy, dz]);

    const supportResult = await runWorker<SupportResponse>(supportWorker, {
      type: 'support',
      positions: meshed.positions,
      normals: meshed.normals,
      areas: meshed.areas,
      direction,
      config: config.support!,
    });
    supportWorker.terminate();
    supports = supportResult.supports;
  }

  onProgress('Done', 100);

  return {
    positions: meshed.positions,
    normals: meshed.normals,
    areas: meshed.areas,
    candidates: scoreResult.candidates,
    supports,
  };
}
