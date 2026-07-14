import { init, prepare_data } from '../pkg/orient_core.js';
import type { OriData } from './types';
import { MAX_FILE_BYTES } from './constants';

let wasmReady = false;

export async function initWasm(): Promise<void> {
  await init();
  wasmReady = true;
  console.log('WASM initialized');
}

export async function loadSTLBytes(file: File): Promise<Uint8Array> {
  if (file.size === 0) throw new Error('Empty file');
  if (file.size > MAX_FILE_BYTES) throw new Error('File too large (>100MB)');
  const buffer = await file.arrayBuffer();
  return new Uint8Array(buffer);
}

export function prepareData(bytes: Uint8Array, config: Record<string, unknown>): OriData {
  if (!wasmReady) throw new Error('WASM not initialized');
  return prepare_data(bytes, config) as OriData;
}
