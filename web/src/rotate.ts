/**
 * Rotate every vertex in `positions` by the unit quaternion `q`.
 *
 * Convention: q = [qx, qy, qz, qw]  (xyzw — three.js standard).
 *
 * This is the export-path counterpart to the viewport's `mesh.quaternion`
 * assignment. The exported STL must match the on-screen rotation, so the math
 * here must agree with `THREE.Quaternion.applyToVector3Array` to float
 * epsilon. See rotate.test.ts for the parity test that enforces this.
 *
 * Extracted from main.ts so it can be unit-tested in isolation.
 */
export function rotatePositions(
  positions: Float32Array,
  q: [number, number, number, number],
): Float32Array {
  const out = new Float32Array(positions.length);
  const qx = q[0], qy = q[1], qz = q[2], qw = q[3];
  for (let i = 0; i < positions.length; i += 3) {
    const x = positions[i], y = positions[i + 1], z = positions[i + 2];
    // Canonical quaternion rotation v' = v + qw*t + (q.xyz × t),
    // where t = 2 * (q.xyz × v).  Equivalent to q ⊗ v ⊗ q* for a unit
    // quaternion, and bit-identical to THREE.Quaternion.applyToVector3.
    const tx = 2 * (qy * z - qz * y);
    const ty = 2 * (qz * x - qx * z);
    const tz = 2 * (qx * y - qy * x);
    out[i] = x + qw * tx + (qy * tz - qz * ty);
    out[i + 1] = y + qw * ty + (qz * tx - qx * tz);
    out[i + 2] = z + qw * tz + (qx * ty - qy * tx);
  }
  return out;
}
