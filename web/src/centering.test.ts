import { describe, it, expect } from "vitest";
import { centroidTranslate, boundingRadius } from "./centering";

/**
 * Tests for the centroid-bake approach to candidate centering.
 *
 * The bug: the mesh rotated around its geometry's local (0,0,0), which is an
 * arbitrary corner — NOT the centroid. The mesh orbited instead of spinning in
 * place, so poses drifted off-center per candidate.
 *
 * The fix: bake the centroid-centering INTO the geometry via translate(), so
 * the local origin IS the centroid. Then mesh.quaternion rotates around the
 * centroid, and modelGroup only needs a Y-lift to sit on the plate.
 */
describe("centroidTranslate", () => {
  it("moves a non-origin centroid to the local origin", () => {
    const centroid = { x: 5, y: -3, z: 2 };
    const offset = centroidTranslate(centroid);

    // Applying the bake to the centroid yields the origin.
    expect(centroid.x + offset.x).toBeCloseTo(0, 6);
    expect(centroid.y + offset.y).toBeCloseTo(0, 6);
    expect(centroid.z + offset.z).toBeCloseTo(0, 6);
  });

  it("keeps a centroid already at origin at the origin (no-op)", () => {
    const offset = centroidTranslate({ x: 0, y: 0, z: 0 });
    // toBeCloseTo treats -0 and 0 as equal (Object.is does not).
    expect(offset.x).toBeCloseTo(0, 6);
    expect(offset.y).toBeCloseTo(0, 6);
    expect(offset.z).toBeCloseTo(0, 6);
  });

  it("preserves the invariant across many centroids (regression: orbit bug)", () => {
    // For ANY centroid, baking it must land the centroid at origin — that is
    // what makes rotation happen around the mesh center, not a corner.
    const cases = [
      { x: 2.5, y: 2.5, z: 2.5 },
      { x: -100, y: 0.001, z: 42 },
      { x: 1e6, y: -1e6, z: 0 },
    ];
    for (const c of cases) {
      const o = centroidTranslate(c);
      expect(c.x + o.x).toBeCloseTo(0, 6);
      expect(c.y + o.y).toBeCloseTo(0, 6);
      expect(c.z + o.z).toBeCloseTo(0, 6);
    }
  });
});

describe("boundingRadius", () => {
  it("returns the max distance from centroid to any vertex", () => {
    const positions = new Float32Array([3, 4, 0, 0, 0, 0]);
    const centroid = { x: 1.5, y: 2, z: 0 };
    expect(boundingRadius(centroid, positions)).toBeCloseTo(2.5, 5);
  });

  it("handles centroid at the origin", () => {
    const positions = new Float32Array([0, 5, 0, 3, 0, 4, 0, 0, 0]);
    expect(boundingRadius({ x: 0, y: 0, z: 0 }, positions)).toBeCloseTo(5, 5);
  });

  it("returns 0 for a single point at the centroid", () => {
    const positions = new Float32Array([2, 3, 4]);
    expect(boundingRadius({ x: 2, y: 3, z: 4 }, positions)).toBe(0);
  });

  it("returns 0 for empty positions", () => {
    expect(boundingRadius({ x: 0, y: 0, z: 0 }, new Float32Array(0))).toBe(0);
  });
});


