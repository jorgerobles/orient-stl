import type { OriData, ComputeConfig, Candidate, WorkerMessage, WorkerRequest } from './types';

let wasmReady: Promise<any> | null = null;

async function ensureWasm() {
  if (!wasmReady) {
    wasmReady = import('../pkg/orient_core.js').then((mod: any) => {
      if (mod.init) mod.init();
      return mod;
    }).catch((err) => {
      console.warn('WASM refine unavailable:', err);
      return null;
    });
  }
  return wasmReady;
}

self.onmessage = async (e: MessageEvent<WorkerRequest>) => {
  const { data, config, weights, ranker, maxCandidates, minAngleDeg, normLo, normHi } = e.data;

  const wasm = await ensureWasm();
  if (!wasm) { self.postMessage({ type: 'error', message: 'WASM not loaded' } satisfies WorkerMessage); return; }

  const progressFn = (i: number, t: number) =>
    self.postMessage({ type: 'progress', value: Math.round(i / t * 100) } satisfies WorkerMessage);

  // Prepend identity direction [0,-1,0] so the as-loaded orientation is scored
  // with the SAME refinement as hull candidates (not 0 iterations)
  const identityDir = new Float32Array([0, -1, 0]);
  const allDirections = new Float32Array(identityDir.length + data.directions.length);
  allDirections.set(identityDir, 0);
  allDirections.set(data.directions, identityDir.length);

  // Score all directions (identity + hull) with identical refinement
  const metrics = wasm.score_all_directions(
    data.positions, data.normals, data.areas, allDirections,
    config.criticalAngleDeg, config.refineIterations ?? 0, config.excludeUnstable, progressFn,
  ) as Float32Array;

  const dirCount = allDirections.length / 3;
  const stableFlags = new Float32Array(dirCount);
  for (let i = 0; i < dirCount; i++) stableFlags[i] = metrics[i * 13 + 10];

  const ranked = wasm.rank_candidates(
    metrics, new Float32Array(weights), ranker,
    new Float32Array(normLo ?? []), new Float32Array(normHi ?? []),
  ) as Float32Array;
  const selected = wasm.select_diverse(
    ranked, allDirections, stableFlags,
    config.excludeUnstable, maxCandidates ?? config.maxCandidates, minAngleDeg ?? 15,
  ) as Float32Array;

  const scoreMap = new Map<number, number>();
  for (let i = 0; i < ranked.length; i += 2) scoreMap.set(ranked[i], ranked[i + 1]);

  // Floor: original orientation score — reject any candidate below it
  const identityScore = scoreMap.get(0) ?? 0;

  const candidates: Candidate[] = [];
  for (let si = 0; si < selected.length; si++) {
    const idx = selected[si], base = idx * 13;
    const score = scoreMap.get(idx) ?? 0;
    if (score < identityScore) continue;
    candidates.push({
      id: `candidate-${idx}`,
      quaternion: [metrics[base], metrics[base + 1], metrics[base + 2], metrics[base + 3]],
      overhangPenalty: metrics[base + 4], footprint: metrics[base + 5], maxCross: metrics[base + 6],
      shadowed: metrics[base + 9], surfaceQuality: metrics[base + 7], estHeight: metrics[base + 8],
      refinedOverhang: metrics[base + 4], refineVariance: 0,
      stability: metrics[base + 10] > 0.5 ? 'stable' : 'unstable',
      stabilityMargin: metrics[base + 11], contactArea: metrics[base + 12],
      compositeScore: score,
    });
  }

  self.postMessage({ type: 'results', candidates } satisfies WorkerMessage);
};
