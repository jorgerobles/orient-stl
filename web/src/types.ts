export interface OrientConfig {
  mode: "hull" | "hull_plus_sphere";
  criticalAngleDeg: number;
  dedupeAngleDeg: number;
  refineIterations: number;
  excludeUnstable: boolean;
  maxCandidates: number;
  maxHoleEdges: number;
  weldEpsilon: number;
}

export function defaultConfig(): OrientConfig {
  return {
    mode: "hull",
    criticalAngleDeg: 30,
    dedupeAngleDeg: 3,
    refineIterations: 50,
    excludeUnstable: true,
    maxCandidates: 20,
    maxHoleEdges: 0,
    weldEpsilon: 0,
  };
}

// ─── Shared data types (moved from compute.ts) ───

export interface OriData {
  positions: Float32Array;
  normals: Float32Array;
  areas: Float32Array;
  directions: Float32Array;
}

export interface Candidate {
  id: string;
  quaternion: [number, number, number, number];
  overhangPenalty: number;
  footprint: number;
  maxCross: number;
  shadowed: number;
  surfaceQuality: number;
  /** Height risk = raw_height_mm × overhang_penalty (derived metric) */
  estHeight: number;
  refinedOverhang: number;
  refineVariance: number;
  stability: 'stable' | 'unstable';
  stabilityMargin: number;
  contactArea: number;
  compositeScore: number;
}

export interface ComputeConfig {
  criticalAngleDeg: number;
  excludeUnstable: boolean;
  maxCandidates: number;
  refineIterations?: number;
}

/** Weight configuration for the composite score. Each weight scales a
 *  min-max-normalised component (cost form, lower = better). `wSurface` and
 *  `wHeight` are included so every heuristic can be tuned per use case. */
export interface ScoreWeights {
  wOverhang: number;
  wFootprint: number;
  wCross: number;
  wSurface: number;
  wHeight: number;
  wShadowed: number;
}

// ─── Worker message types (defined but not applied until Plan 03) ───

export type WorkerMessage =
  | { readonly type: 'progress'; readonly value: number }
  | { readonly type: 'results'; readonly candidates: Candidate[] }
  | { readonly type: 'error'; readonly message: string };

export interface WorkerRequest {
  data: OriData;
  config: ComputeConfig;
  weights: [number, number, number, number, number, number];
  ranker: string;
  maxCandidates: number;
  minAngleDeg: number;
  normLo: number[] | null;
  normHi: number[] | null;
}
