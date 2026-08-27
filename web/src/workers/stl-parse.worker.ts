/// <reference lib="webworker" />

(() => {
  interface ParseRequestMsg {
    type: 'parse';
    bytes: Uint8Array;
  }

  interface ParseResponseMsg {
    type: 'result';
    positions: Float32Array;
  }

  interface ErrorResponseMsg {
    type: 'error';
    message: string;
  }

  type WRequest = ParseRequestMsg;
  type WResponse = ParseResponseMsg | ErrorResponseMsg;

  let wasmReady: Promise<any> | null = null;

  async function ensureWasm() {
    if (!wasmReady) {
      wasmReady = import('../../pkg/stl-parse/stl_parse.js').then((mod: any) => {
        return mod;
      }).catch((err) => {
        console.error('STL parse WASM load failed:', err);
        return null;
      });
    }
    return wasmReady;
  }

  self.onmessage = async (e: MessageEvent<WRequest>) => {
    try {
      const wasm = await ensureWasm();
      if (!wasm) {
        self.postMessage({ type: 'error', message: 'STL parse WASM not loaded' } satisfies WResponse);
        return;
      }

      const positions = wasm.parse_stl_wasm(e.data.bytes) as Float32Array;
      self.postMessage({ type: 'result', positions } satisfies WResponse, [positions.buffer]);
    } catch (err: any) {
      self.postMessage({ type: 'error', message: err?.message ?? String(err) } satisfies WResponse);
    }
  };
})();
