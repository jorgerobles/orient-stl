/// <reference lib="webworker" />

(() => {
  interface RepairRequestMsg {
    type: 'repair';
    positions: Float32Array;
    weldEpsilon: number;
    maxHoleEdges: number;
  }

  interface RepairResponseMsg {
    type: 'result';
    positions: Float32Array;
  }

  interface ErrorResponseMsg {
    type: 'error';
    message: string;
  }

  type WRequest = RepairRequestMsg;
  type WResponse = RepairResponseMsg | ErrorResponseMsg;

  let wasmReady: Promise<any> | null = null;

  async function ensureWasm() {
    if (!wasmReady) {
      wasmReady = import('../../pkg/stl-repair/stl_repair.js').then((mod: any) => {
        return mod;
      }).catch((err) => {
        console.error('STL repair WASM load failed:', err);
        return null;
      });
    }
    return wasmReady;
  }

  self.onmessage = async (e: MessageEvent<WRequest>) => {
    try {
      const wasm = await ensureWasm();
      if (!wasm) {
        self.postMessage({ type: 'error', message: 'STL repair WASM not loaded' } satisfies WResponse);
        return;
      }

      const { positions, weldEpsilon, maxHoleEdges } = e.data;
      const repaired = wasm.repair_mesh_wasm(positions, weldEpsilon, maxHoleEdges) as Float32Array;
      self.postMessage({ type: 'result', positions: repaired } satisfies WResponse, [repaired.buffer]);
    } catch (err: any) {
      self.postMessage({ type: 'error', message: err?.message ?? String(err) } satisfies WResponse);
    }
  };
})();
