import { describe, it, expect } from "vitest";
import { applyConvention } from "./convention";
import type { LoadConvention } from "./convention";

/**
 * Tests for the load-time axis-convention switch.
 *
 * The tool's internal frame is Y-up: the build plate lies in the XZ plane and
 * +Y is the plate normal (see centering.ts → centroidTranslate). STL files arrive
 * in one of two conventions, selected by the user at load time:
 *
 *   'z-up' — STL's +Z is the vertical axis (most CAD output, default for
 *            most slicers). Rotate -90° about X so STL+Z → tool+Y:
 *            (x, y, z) → (x, z, -y).
 *
 *   'y-up' — STL's +Y is already the vertical axis (e.g. Blender, three.js
 *            exports). No rotation needed.
 *
 * The transform applies uniformly to positions, face normals, AND candidate
 * direction vectors — all are flat xyz arrays in the same frame, so the same
 * per-vertex swap keeps the scoring pipeline consistent.
 */
describe("applyConvention", () => {
  describe("'y-up' (already in tool frame — identity)", () => {
    it("returns the input array unchanged (no copy needed for a no-op)", () => {
      const positions = new Float32Array([1, 2, 3, 4, 5, 6]);
      const out = applyConvention(positions, "y-up");
      expect(out).toBe(positions); // same reference — true no-op
      expect(Array.from(out)).toEqual([1, 2, 3, 4, 5, 6]);
    });

    it("handles empty arrays", () => {
      const out = applyConvention(new Float32Array(0), "y-up");
      expect(out.length).toBe(0);
    });
  });

  describe("'z-up' (STL+Z → tool+Y via -90° X rotation)", () => {
      it("maps (x, y, z) → (x, z, -y) for every vertex", () => {
        const positions = new Float32Array([
          1, 2, 3,
          -4, 5, -6,
          0, 0, 0,
          0.1, -0.2, 0.3,
        ]);
        const out = applyConvention(positions, "z-up");
        // (x, y, z) → (x, z, -y), element-wise.
        const expected = [
          1, 3, -2,
          -4, -6, -5,
          0, 0, 0,
          0.1, 0.3, 0.2,
        ];
        expect(out.length).toBe(expected.length);
        for (let i = 0; i < expected.length; i++) {
          expect(out[i]).toBeCloseTo(expected[i], 6);
        }
      });

      it("sends the +Z axis onto +Y (stands a Z-up model upright)", () => {
        // A vertex one unit up the STL's Z axis ends up one unit up the
        // tool's Y axis after the load-time swap.
        const out = applyConvention(new Float32Array([0, 0, 1]), "z-up");
        expect(out[0]).toBeCloseTo(0, 6);
        expect(out[1]).toBeCloseTo(1, 6); // old Z → new Y
        expect(out[2]).toBeCloseTo(0, 6);
      });

      it("sends the +Y axis onto -Z (preserves handedness, no mirror)", () => {
        const out = applyConvention(new Float32Array([0, 1, 0]), "z-up");
        expect(out[0]).toBeCloseTo(0, 6);
        expect(out[1]).toBeCloseTo(0, 6);
        expect(out[2]).toBeCloseTo(-1, 6); // old Y → new -Z
      });

      it("preserves vector lengths (orthogonal — no shear/scale)", () => {
        const positions = new Float32Array([3, 4, 0, 1, 1, 1, -2, 5, 1]);
        const out = applyConvention(positions, "z-up");
        expect(Math.hypot(out[0], out[1], out[2])).toBeCloseTo(5, 6);
        expect(Math.hypot(out[3], out[4], out[5])).toBeCloseTo(Math.sqrt(3), 6);
        expect(Math.hypot(out[6], out[7], out[8])).toBeCloseTo(Math.sqrt(30), 6);
      });

      it("does not mutate the input array", () => {
        const positions = new Float32Array([1, 2, 3, -4, 5, -6]);
        const snapshot = Array.from(positions);
        applyConvention(positions, "z-up");
        expect(Array.from(positions)).toEqual(snapshot);
      });

      it("returns a new array (not the input reference)", () => {
        const positions = new Float32Array([1, 2, 3]);
        const out = applyConvention(positions, "z-up");
        expect(out).not.toBe(positions);
      });

      it("works equally for normal vectors and direction vectors (same math)", () => {
        // Normals and candidate directions are 3-vectors just like positions;
        // the swap is identical. This documents that the caller may pass any
        // flat xyz array, not just vertex positions.
        const normals = new Float32Array([0, 0, 1, 0, 1, 0, 1, 0, 0]);
        const out = applyConvention(normals, "z-up");
        const expected = [0, 1, 0, 0, 0, -1, 1, 0, 0];
        expect(out.length).toBe(expected.length);
        for (let i = 0; i < expected.length; i++) {
          expect(out[i]).toBeCloseTo(expected[i], 6);
        }
      });

      it("handles empty arrays", () => {
        const out = applyConvention(new Float32Array(0), "z-up");
        expect(out.length).toBe(0);
      });
  });

  it("exhaustive: every LoadConvention value produces a length-preserving result", () => {
    const conventions: LoadConvention[] = ["z-up", "y-up"];
    const positions = new Float32Array([3, 4, 0, 1, 2, 2]);
    for (const conv of conventions) {
      const out = applyConvention(positions, conv);
      expect(out.length).toBe(positions.length);
      for (let i = 0; i < positions.length; i += 3) {
        const inLen = Math.hypot(positions[i], positions[i + 1], positions[i + 2]);
        const outLen = Math.hypot(out[i], out[i + 1], out[i + 2]);
        expect(outLen).toBeCloseTo(inLen, 6);
      }
    }
  });
});
