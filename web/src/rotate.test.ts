import { describe, it, expect } from "vitest";
import { rotatePositions } from "./rotate";
import * as THREE from "three";

/**
 * Tests for the export-path quaternion rotation.
 *
 * The bug (main.ts:378-388, now extracted to rotate.ts): the original
 * `rotatePositions` used a scrambled sandwich-product formula. The dot-product
 * slot wrongly used q[3] (= q.w, the SCALAR) where it should have used q[2]
 * (= q.z, the vector z-component), and the output-axis assembly terms did not
 * match any standard form of q ⊗ v ⊗ q*. The resulting linear map was
 * non-orthogonal: it sheared/squished the mesh instead of rotating it. Even
 * the IDENTITY quaternion did not round-trip (it produced a Z mirror).
 *
 * The fix: replace the body with the canonical cross-product form
 *   t = 2 (q.xyz × v)
 *   v' = v + q.w t + (q.xyz × t)
 * which is verified to round-trip identity and to match three.js'
 * Quaternion.applyToVector3Array, so the export path now matches the viewport.
 *
 * Convention: q = [qx, qy, qz, qw]  (xyzw, three.js standard).
 */
describe("rotatePositions", () => {
  it("round-trips the identity quaternion [0,0,0,1] (no mirror, no shear)", () => {
    const positions = new Float32Array([
      1, 2, 3,
      -4, 0.5, 7,
      0, -1, -2,
      100, -200, 300,
    ]);
    const out = rotatePositions(positions, [0, 0, 0, 1]);
    for (let i = 0; i < positions.length; i++) {
      expect(out[i]).toBeCloseTo(positions[i], 6);
    }
  });

  it("180° about Y [0,1,0,0] sends (x,y,z) -> (-x, y, -z)", () => {
    const positions = new Float32Array([
      1, 2, 3,
      -4, 5, -6,
      0.1, -0.2, 0.3,
    ]);
    const out = rotatePositions(positions, [0, 1, 0, 0]);
    // Vertex 0: (1,2,3) -> (-1, 2, -3)
    expect(out[0]).toBeCloseTo(-1, 6);
    expect(out[1]).toBeCloseTo(2, 6);
    expect(out[2]).toBeCloseTo(-3, 6);
    // Vertex 1: (-4,5,-6) -> (4, 5, 6)
    expect(out[3]).toBeCloseTo(4, 6);
    expect(out[4]).toBeCloseTo(5, 6);
    expect(out[5]).toBeCloseTo(6, 6);
    // Vertex 2: (0.1,-0.2,0.3) -> (-0.1, -0.2, -0.3)
    expect(out[6]).toBeCloseTo(-0.1, 6);
    expect(out[7]).toBeCloseTo(-0.2, 6);
    expect(out[8]).toBeCloseTo(-0.3, 6);
  });

  it("180° about X [1,0,0,0] sends (x,y,z) -> (x, -y, -z)", () => {
    const positions = new Float32Array([1, 2, 3]);
    const out = rotatePositions(positions, [1, 0, 0, 0]);
    expect(out[0]).toBeCloseTo(1, 6);
    expect(out[1]).toBeCloseTo(-2, 6);
    expect(out[2]).toBeCloseTo(-3, 6);
  });

  it("180° about Z [0,0,1,0] sends (x,y,z) -> (-x, -y, z)", () => {
    const positions = new Float32Array([1, 2, 3]);
    const out = rotatePositions(positions, [0, 0, 1, 0]);
    expect(out[0]).toBeCloseTo(-1, 6);
    expect(out[1]).toBeCloseTo(-2, 6);
    expect(out[2]).toBeCloseTo(3, 6);
  });

  it("90° about Y maps the X axis onto the -Z axis (right-handed)", () => {
    // q for 90° about Y: angle=π/2 → w=cos(π/4), y=sin(π/4)
    const s = Math.SQRT1_2;
    const positions = new Float32Array([1, 0, 0]); // unit X
    const out = rotatePositions(positions, [0, s, 0, s]);
    // Right-handed 90° about +Y sends +X to -Z.
    expect(out[0]).toBeCloseTo(0, 6);
    expect(out[1]).toBeCloseTo(0, 6);
    expect(out[2]).toBeCloseTo(-1, 6);
  });

  it("preserves vector lengths (rotation is orthogonal — the original bug wasn't)", () => {
    // A genuine rotation preserves |v|. The old buggy formula scaled/sheared,
    // so |out| != |in|. This test would have caught the bug directly.
    const positions = new Float32Array([3, 4, 0, 1, 1, 1, -2, 5, 1]);
    const inLen0 = Math.hypot(3, 4, 0);
    const inLen1 = Math.hypot(1, 1, 1);
    const inLen2 = Math.hypot(-2, 5, 1);
    // 30° about an arbitrary axis — exercise all terms.
    const axis = new THREE.Vector3(1, 2, 3).normalize();
    const angle = Math.PI / 6;
    const q = new THREE.Quaternion(
      axis.x * Math.sin(angle / 2),
      axis.y * Math.sin(angle / 2),
      axis.z * Math.sin(angle / 2),
      Math.cos(angle / 2),
    );
    const out = rotatePositions(positions, [q.x, q.y, q.z, q.w]);
    expect(Math.hypot(out[0], out[1], out[2])).toBeCloseTo(inLen0, 6);
    expect(Math.hypot(out[3], out[4], out[5])).toBeCloseTo(inLen1, 6);
    expect(Math.hypot(out[6], out[7], out[8])).toBeCloseTo(inLen2, 6);
  });

  it("matches three.js Quaternion.applyToVector3Array across several rotations", () => {
    // This is the core parity check: the export path must agree with the
    // viewport (which uses three.js) to within float epsilon. If this passes,
    // the exported STL will match what the user sees on screen.
    const positions = new Float32Array([
      12.3, -4.5, 7.8,
      -1.1, 2.2, -3.3,
      0.01, -0.02, 0.03,
      100, 200, 300,
      -50, 60, -70,
    ]);
    const cases: [number, number, number, number][] = [
      [0, 0, 0, 1],                         // identity
      [0, 1, 0, 0],                         // 180° Y
      [1, 0, 0, 0],                         // 180° X
      [0, 0, 1, 0],                         // 180° Z
      [0, Math.SQRT1_2, 0, Math.SQRT1_2],   // 90° Y
      [Math.SQRT1_2, 0, 0, Math.SQRT1_2],   // 90° X
    ];
    for (const [qx, qy, qz, qw] of cases) {
      const out = rotatePositions(positions, [qx, qy, qz, qw]);
      const tq = new THREE.Quaternion(qx, qy, qz, qw);
      const v3 = new THREE.Vector3();
      for (let i = 0; i < positions.length; i += 3) {
        v3.set(positions[i], positions[i + 1], positions[i + 2]).applyQuaternion(tq);
        expect(out[i]).toBeCloseTo(v3.x, 5);
        expect(out[i + 1]).toBeCloseTo(v3.y, 5);
        expect(out[i + 2]).toBeCloseTo(v3.z, 5);
      }
    }
  });

  it("handles empty positions", () => {
    const out = rotatePositions(new Float32Array(0), [0, 0, 0, 1]);
    expect(out.length).toBe(0);
  });

  it("does not mutate the input array", () => {
    const positions = new Float32Array([1, 2, 3, 4, 5, 6]);
    const snapshot = Array.from(positions);
    rotatePositions(positions, [0, 1, 0, 0]);
    expect(Array.from(positions)).toEqual(snapshot);
  });
});
