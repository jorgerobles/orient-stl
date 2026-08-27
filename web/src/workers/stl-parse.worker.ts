/// <reference lib="webworker" />

interface ParseRequest {
  type: 'parse';
  bytes: Uint8Array;
}

interface ParseResponse {
  type: 'result';
  positions: Float32Array;
}

interface ErrorResponse {
  type: 'error';
  message: string;
}

type WorkerRequest = ParseRequest;
type WorkerResponse = ParseResponse | ErrorResponse;

let wasmReady: Promise<any> | null = null;

async function ensureWasm() {
  if (!wasmReady) {
    wasmReady = import('../../pkg/stl-parse/stl_parse.js').then(async (mod: any) => {
      if (mod.init) await mod.init();
      return mod;
    }).catch((err) => {
      console.error('STL parse WASM load failed:', err);
      return null;
    });
  }
  return wasmReady;
}

self.onmessage = async (e: MessageEvent<WorkerRequest>) => {
  try {
    const wasm = await ensureWasm();
    if (!wasm) {
      self.postMessage({ type: 'error', message: 'STL parse WASM not loaded' } satisfies WorkerResponse);
      return;
    }

    const positions = wasm.parse_stl_wasm(e.data.bytes) as Float32Array;
    self.postMessage({ type: 'result', positions } satisfies WorkerResponse, [positions.buffer]);
  } catch (err: any) {
    self.postMessage({ type: 'error', message: err?.message ?? String(err) } satisfies WorkerResponse);
  }
};
