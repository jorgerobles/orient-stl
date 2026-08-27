/// <reference lib="webworker" />

import type { SupportConfig, SupportResult } from '../types';

(() => {
  interface SupportRequestMsg {
    type: 'support';
    positions: Float32Array;
    normals: Float32Array;
    areas: Float32Array;
    direction: Float32Array;
    config: SupportConfig;
  }

  interface SupportResponseMsg {
    type: 'result';
    supports: SupportResult;
  }

  interface ErrorResponseMsg {
    type: 'error';
    message: string;
  }

  type WRequest = SupportRequestMsg;
  type WResponse = SupportResponseMsg | ErrorResponseMsg;

  let wasmReady: Promise<any> | null = null;

  async function ensureWasm() {
    if (!wasmReady) {
      wasmReady = import('../../pkg/support/support.js').then(async (mod: any) => {
        if (mod.init) await mod.init();
        return mod;
      }).catch((err) => {
        console.error('Support WASM load failed:', err);
        return null;
      });
    }
    return wasmReady;
  }

  self.onmessage = async (e: MessageEvent<WRequest>) => {
    const { positions, normals, areas, direction, config } = e.data;

    try {
      const wasm = await ensureWasm();
      if (!wasm) {
        self.postMessage({ type: 'error', message: 'Support WASM not loaded' } satisfies WResponse);
        return;
      }

      const supports: SupportResult = wasm.generate_supports(
        positions, normals, areas, direction, config,
      );

      self.postMessage({ type: 'result', supports } satisfies WResponse);
    } catch (err: any) {
      self.postMessage({ type: 'error', message: err?.message ?? String(err) } satisfies WResponse);
    }
  };
})();
