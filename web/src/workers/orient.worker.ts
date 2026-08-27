/// <reference lib="webworker" />

import type { OriData, ComputeConfig, Candidate } from '../types';

(() => {
  interface ScoreRequestMsg {
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

  interface ScoreResponseMsg {
    type: 'results';
    candidates: Candidate[];
  }

  interface ProgressResponseMsg {
    type: 'progress';
    value: number;
  }

  interface ErrorResponseMsg {
    type: 'error';
    message: string;
  }

  type WRequest = ScoreRequestMsg;
  type WResponse = ScoreResponseMsg | ProgressResponseMsg | ErrorResponseMsg;

  let wasmReady: Promise<any> | null = null;

  async function ensureWasm() {
    if (!wasmReady) {
      wasmReady = import('../../pkg/orient/orient.js').then(async (mod: any) => {
        if (mod.init) await mod.init();
        return mod;
      }).catch((err) => {
        console.error('Orient WASM load failed:', err);
        return null;
      });
    }
    return wasmReady;
  }

  self.onmessage = async (e: MessageEvent<WRequest>) => {
    const { data, config, weights, ranker, maxCandidates, minAngleDeg, normLo, normHi } = e.data;

    try {
      const wasm = await ensureWasm();
      if (!wasm) {
        self.postMessage({ type: 'error', message: 'Orient WASM not loaded' } satisfies WResponse);
        return;
      }

      const progressFn = (i: number, t: number) =>
        self.postMessage({ type: 'progress', value: Math.round(i / t * 100) } satisfies WResponse);

      const identityDir = new Float32Array([0, -1, 0]);
      const allDirections = new Float32Array(identityDir.length + data.directions.length);
      allDirections.set(identityDir, 0);
      allDirections.set(data.directions, identityDir.length);

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

      self.postMessage({ type: 'results', candidates } satisfies WResponse);
    } catch (err: any) {
      self.postMessage({ type: 'error', message: err?.message ?? String(err) } satisfies WResponse);
    }
  };
})();
