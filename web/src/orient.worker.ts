import { computeSlice } from './compute';
import type { OriData, ComputeConfig, RefineFn } from './compute';

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

self.onmessage = async (e: MessageEvent) => {
  const { data, config, dirStart, dirCount } = e.data as {
    data: OriData; config: ComputeConfig; dirStart: number; dirCount: number;
  };
  const wasm = await ensureWasm();
  let refineFn: RefineFn | undefined;
  if (wasm && wasm.refine_orientation_batch) {
    refineFn = (dir, positions, normals, areas, criticalAngleDeg) =>
      wasm.refine_orientation_batch(
        positions, normals, areas,
        dir[0], dir[1], dir[2],
        criticalAngleDeg, config.refineIterations ?? 0, 4, 0,
      ) as number[];
  }
  const results = computeSlice(data, config, dirStart, dirCount, (pct: number) => {
    self.postMessage({ type: 'progress', value: pct });
  }, refineFn);
  self.postMessage({ type: 'slice-done', results, dirStart, dirCount });
};