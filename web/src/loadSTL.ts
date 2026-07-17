import {
  init, stl_parse_to_positions, repair_mesh_tris,
  normalize_winding_tris, weld_vertices_tris, fill_holes_tris,
  compute_mesh_oridata, prepare_data, count_boundary_edges_wasm,
} from '../pkg/orient_core.js';
import type { OriData } from './types';
import { MAX_FILE_BYTES } from './constants';

let wasmReady = false;

const yieldToBrowser = () => new Promise<void>(r => setTimeout(r, 0));

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

export type ProgressCallback = (label: string, pct: number) => void;

export function prepareData(bytes: Uint8Array, config: Record<string, unknown>): OriData {
  if (!wasmReady) throw new Error('WASM not initialized');
  return prepare_data(bytes, config) as OriData;
}

async function step(label: string, pct: number, onProgress: ProgressCallback): Promise<void> {
  onProgress(label, pct);
  await yieldToBrowser();
}

export async function loadWithProgress(
  bytes: Uint8Array,
  autoRepair: boolean,
  onProgress: ProgressCallback,
): Promise<OriData> {
  if (!wasmReady) throw new Error('WASM not initialized');

  const weldEpsilon = 1e-5;
  const maxHoleEdges = 64;

  performance.mark('load-start');

  await step('Parsing STL...', 5, onProgress);
  const positions = stl_parse_to_positions(bytes) as Float32Array;
  performance.mark('load-parse');

  if (autoRepair) {
    await step('Removing duplicate triangles...', 15, onProgress);
    let p = repair_mesh_tris(positions) as Float32Array;
    performance.mark('load-repair');

    await step('Fixing triangle winding...', 30, onProgress);
    p = normalize_winding_tris(p) as Float32Array;
    performance.mark('load-winding');

    await step('Welding nearby vertices...', 45, onProgress);
    p = weld_vertices_tris(p, weldEpsilon) as Float32Array;
    performance.mark('load-weld');

    await step('Removing post-weld duplicates...', 55, onProgress);
    p = repair_mesh_tris(p) as Float32Array;
    performance.mark('load-postweld');

    await step('Filling holes in mesh...', 65, onProgress);
    p = fill_holes_tris(p, maxHoleEdges) as Float32Array;
    performance.mark('load-fill');

    await step('Computing hull & candidates...', 80, onProgress);
    const raw = compute_mesh_oridata(p, 'hull', 3.0) as any;
    const result: OriData = {
      positions: new Float32Array(raw.positions),
      normals: new Float32Array(raw.normals),
      areas: new Float32Array(raw.areas),
      directions: new Float32Array(raw.directions),
    };
    performance.mark('load-hull');

    const boundaryEdges = count_boundary_edges_wasm(p) as number;

    performance.mark('load-end');
    performance.measure('load-total', 'load-start', 'load-end');
    performance.measure('load-parse-only', 'load-start', 'load-parse');
    performance.measure('load-repair-only', 'load-parse', 'load-repair');
    performance.measure('load-winding-only', 'load-repair', 'load-winding');
    performance.measure('load-weld-only', 'load-winding', 'load-weld');
    performance.measure('load-fill-only', 'load-postweld', 'load-fill');
    performance.measure('load-hull-only', 'load-fill', 'load-end');

    const entries = performance.getEntriesByType('measure');
    const total = entries.find(e => e.name === 'load-total');
    if (total) {
      console.log(`[perf] load total: ${(total.duration / 1000).toFixed(2)}s`);
      console.log(`[perf]   parse: ${(entries.find(e => e.name === 'load-parse-only')?.duration ?? 0).toFixed(0)}ms`);
      console.log(`[perf]   repair: ${(entries.find(e => e.name === 'load-repair-only')?.duration ?? 0).toFixed(0)}ms`);
      console.log(`[perf]   winding: ${(entries.find(e => e.name === 'load-winding-only')?.duration ?? 0).toFixed(0)}ms`);
      console.log(`[perf]   weld: ${(entries.find(e => e.name === 'load-weld-only')?.duration ?? 0).toFixed(0)}ms`);
      console.log(`[perf]   fill: ${(entries.find(e => e.name === 'load-fill-only')?.duration ?? 0).toFixed(0)}ms`);
      console.log(`[perf]   hull+candidates: ${(entries.find(e => e.name === 'load-hull-only')?.duration ?? 0).toFixed(0)}ms`);
      console.log(`[perf]   final tris: ${result.positions.length / 9}  boundary edges: ${boundaryEdges}`);
    }
    performance.clearMarks('load-start');
    performance.clearMarks('load-parse');
    performance.clearMarks('load-repair');
    performance.clearMarks('load-winding');
    performance.clearMarks('load-weld');
    performance.clearMarks('load-postweld');
    performance.clearMarks('load-fill');
    performance.clearMarks('load-hull');
    performance.clearMarks('load-end');
    performance.clearMeasures('load-total');
    performance.clearMeasures('load-parse-only');
    performance.clearMeasures('load-repair-only');
    performance.clearMeasures('load-winding-only');
    performance.clearMeasures('load-weld-only');
    performance.clearMeasures('load-fill-only');
    performance.clearMeasures('load-hull-only');

    return result;
  } else {
    await step('Computing hull & candidates...', 30, onProgress);
    const raw = compute_mesh_oridata(positions, 'hull', 3.0) as any;
    const result: OriData = {
      positions: new Float32Array(raw.positions),
      normals: new Float32Array(raw.normals),
      areas: new Float32Array(raw.areas),
      directions: new Float32Array(raw.directions),
    };
    performance.mark('load-end');
    performance.measure('load-total', 'load-start', 'load-end');
    const total = performance.getEntriesByType('measure').find(e => e.name === 'load-total');
    if (total) {
      console.log(`[perf] load total (no repair): ${(total.duration / 1000).toFixed(2)}s`);
    }
    performance.clearMarks('load-start');
    performance.clearMarks('load-end');
    performance.clearMeasures('load-total');
    return result;
  }
}
