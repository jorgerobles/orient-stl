import { init, compute_orientations } from '../web/pkg/orient_core.js';
import fs from 'fs';

const stl = fs.readFileSync('test-tetrahedron.stl');
console.log(`STL file: ${stl.length} bytes`);

await init();
console.log('WASM initialized');

try {
  const config = {
    mode: 'hull',
    criticalAngleDeg: 30,
    dedupeAngleDeg: 3,
    refineIterations: 0,
    excludeUnstable: true,
  };
  const result = compute_orientations(new Uint8Array(stl), config);
  console.log('Result:', JSON.stringify(result, null, 2));
} catch (err) {
  console.error('Error:', err);
}
