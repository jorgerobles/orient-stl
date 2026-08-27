import { runPipeline } from './pipeline';
import type { PipelineConfig, PipelineResult } from './pipeline';
import type { OriData, Candidate, ComputeConfig } from './types';
import { MAX_FILE_BYTES, DEFAULT_PROFILE, MIN_ANGLE_DEG } from './constants';
import { WEIGHT_PRESETS } from './profiles';
import { DEFAULT_RANKER } from './constants';
import { compute_directions } from '../pkg/orient/orient.js';

export type ProgressCallback = (label: string, pct: number) => void;

export async function loadSTLBytes(file: File): Promise<Uint8Array> {
  if (file.size === 0) throw new Error('Empty file');
  if (file.size > MAX_FILE_BYTES) throw new Error('File too large (>100MB)');
  const buffer = await file.arrayBuffer();
  return new Uint8Array(buffer);
}

export async function loadWithProgress(
  bytes: Uint8Array,
  autoRepair: boolean,
  onProgress: ProgressCallback,
): Promise<OriData & { candidates: Candidate[] }> {
  const config = autoRepair ? buildRepairConfig() : { ...buildRepairConfig(), autoRepair: false };
  const result = await runPipeline(bytes, config, onProgress);

  if (result.positions.length === 0) throw new Error('No triangles in STL');

  const directions = compute_directions(result.positions, 3.0);

  return {
    positions: result.positions,
    normals: result.normals,
    areas: result.areas,
    directions,
    candidates: result.candidates,
  };
}

function buildRepairConfig(): PipelineConfig {
  const w = WEIGHT_PRESETS[DEFAULT_PROFILE] ?? WEIGHT_PRESETS['resin-biased'];
  const weights: [number, number, number, number, number, number] = [
    w.wOverhang, w.wFootprint, w.wCross, w.wSurface, w.wHeight, w.wShadowed,
  ];

  return {
    autoRepair: false,
    weldEpsilon: 1e-5,
    maxHoleEdges: 512,
    orient: {
      criticalAngleDeg: 30,
      excludeUnstable: true,
      maxCandidates: 20,
      refineIterations: 50,
    },
    weights,
    ranker: DEFAULT_RANKER,
    maxCandidates: 20,
    minAngleDeg: MIN_ANGLE_DEG,
  };
}
