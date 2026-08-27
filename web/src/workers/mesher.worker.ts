/// <reference lib="webworker" />

interface MeshRequest {
  type: 'mesh';
  positions: Float32Array;
}

interface MeshResponse {
  type: 'result';
  positions: Float32Array;
  normals: Float32Array;
  areas: Float32Array;
}

interface ErrorResponse {
  type: 'error';
  message: string;
}

type WorkerRequest = MeshRequest;
type WorkerResponse = MeshResponse | ErrorResponse;

let wasmReady: Promise<any> | null = null;

async function ensureWasm() {
  if (!wasmReady) {
    wasmReady = import('../../pkg/mesher/mesher.js').then(async (mod: any) => {
      if (mod.init) await mod.init();
      return mod;
    }).catch((err) => {
      console.error('Mesher WASM load failed:', err);
      return null;
    });
  }
  return wasmReady;
}

self.onmessage = async (e: MessageEvent<WorkerRequest>) => {
  try {
    const wasm = await ensureWasm();
    if (!wasm) {
      self.postMessage({ type: 'error', message: 'Mesher WASM not loaded' } satisfies WorkerResponse);
      return;
    }

    const raw = wasm.precompute_mesh_data_wasm(e.data.positions) as any;
    const result: MeshResponse = {
      type: 'result',
      positions: new Float32Array(raw.positions),
      normals: new Float32Array(raw.normals),
      areas: new Float32Array(raw.areas),
    };
    self.postMessage(result, [
      result.positions.buffer,
      result.normals.buffer,
      result.areas.buffer,
    ]);
  } catch (err: any) {
    self.postMessage({ type: 'error', message: err?.message ?? String(err) } satisfies WorkerResponse);
  }
};
