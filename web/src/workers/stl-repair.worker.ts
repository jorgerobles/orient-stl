/// <reference lib="webworker" />

interface RepairRequest {
  type: 'repair';
  positions: Float32Array;
  weldEpsilon: number;
  maxHoleEdges: number;
}

interface RepairResponse {
  type: 'result';
  positions: Float32Array;
}

interface ErrorResponse {
  type: 'error';
  message: string;
}

type WorkerRequest = RepairRequest;
type WorkerResponse = RepairResponse | ErrorResponse;

let wasmReady: Promise<any> | null = null;

async function ensureWasm() {
  if (!wasmReady) {
    wasmReady = import('../../pkg/stl-repair/stl_repair.js').then(async (mod: any) => {
      if (mod.init) await mod.init();
      return mod;
    }).catch((err) => {
      console.error('STL repair WASM load failed:', err);
      return null;
    });
  }
  return wasmReady;
}

self.onmessage = async (e: MessageEvent<WorkerRequest>) => {
  try {
    const wasm = await ensureWasm();
    if (!wasm) {
      self.postMessage({ type: 'error', message: 'STL repair WASM not loaded' } satisfies WorkerResponse);
      return;
    }

    const { positions, weldEpsilon, maxHoleEdges } = e.data;
    const repaired = wasm.repair_mesh_wasm(positions, weldEpsilon, maxHoleEdges) as Float32Array;
    self.postMessage({ type: 'result', positions: repaired } satisfies WorkerResponse, [repaired.buffer]);
  } catch (err: any) {
    self.postMessage({ type: 'error', message: err?.message ?? String(err) } satisfies WorkerResponse);
  }
};
