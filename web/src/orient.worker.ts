import { computeSlice } from './compute';
import type { OriData, ComputeConfig } from './compute';

self.onmessage = (e: MessageEvent) => {
  const { data, config, dirStart, dirCount } = e.data as {
    data: OriData; config: ComputeConfig; dirStart: number; dirCount: number;
  };
  const results = computeSlice(data, config, dirStart, dirCount, (pct: number) => {
    self.postMessage({ type: 'progress', value: pct });
  });
  self.postMessage({ type: 'slice-done', results, dirStart, dirCount });
};
