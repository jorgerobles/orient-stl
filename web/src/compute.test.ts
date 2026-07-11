import { describe, it, expect } from "vitest";
import { footprintArea, maxCrossSection, shadowedOverhangFraction, rankByWeights, rankByConsensus } from "./compute";
import type { Candidate } from "./compute";

// Unit square in XY plane: two triangles, normal +Z, total area 1.0.
// Per-triangle vertex layout (9 coords per triangle): v0,v1,v2 flat.
function unitSquareData() {
  const normals = new Float32Array([0, 0, 1, 0, 0, 1]); // two triangles, both +Z
  const areas = new Float32Array([0.5, 0.5]);
  // Per-triangle verts: tri0 = (0,0,0)(1,0,0)(1,1,0), tri1 = (0,0,0)(1,1,0)(0,1,0)
  const positions = new Float32Array([
    0, 0, 0, 1, 0, 0, 1, 1, 0,
    0, 0, 0, 1, 1, 0, 0, 1, 0,
  ]);
  return { normals, areas, positions };
}

describe("footprintArea", () => {
  it("is full area when face-on to the projection direction", () => {
    const { normals, areas } = unitSquareData();
    // dir = +Z: projected area = full 1.0
    expect(footprintArea([0, 0, 1], normals, areas)).toBeCloseTo(1.0, 5);
  });
  it("is ~0 when edge-on", () => {
    const { normals, areas } = unitSquareData();
    expect(footprintArea([1, 0, 0], normals, areas)).toBeCloseTo(0, 5);
  });
  it("is reduced by |cos| at 45°", () => {
    const { normals, areas } = unitSquareData();
    const inv = Math.SQRT1_2;
    expect(footprintArea([inv, 0, inv], normals, areas)).toBeCloseTo(inv, 5);
  });
});

describe("maxCrossSection", () => {
  it("concentrates a flat slab into one bin ≈ total area", () => {
    const { positions, normals, areas } = unitSquareData();
    // dir = +Z: all centroids at z=0 → one bin holds everything ≈ 1.0
    expect(maxCrossSection([0, 0, 1], positions, normals, areas, 8)).toBeCloseTo(1.0, 5);
  });
  it("spreads a two-layer shell so max < total", () => {
    const normals = new Float32Array([0, 0, 1, 0, 0, 1]);
    const areas = new Float32Array([0.5, 0.5]);
    // bottom square z=0 and top square z=1
    const positions = new Float32Array([
      0, 0, 0, 1, 0, 0, 1, 1, 0,
      0, 0, 1, 1, 0, 1, 1, 1, 1,
    ]);
    const mx = maxCrossSection([0, 0, 1], positions, normals, areas, 8);
    expect(mx).toBeLessThan(0.75); // each layer ≈ 0.5, not 1.0
  });
  it("returns 0 on empty data", () => {
    expect(maxCrossSection([0, 0, 1], new Float32Array(0), new Float32Array(0), new Float32Array(0), 8)).toBe(0);
  });
});

describe("rankByWeights", () => {
  function mkCand(over: number, foot: number, cross: number, shadowed = 0): Candidate {
    return {
      id: `c-${over}-${foot}-${cross}`,
      quaternion: [1, 0, 0, 0],
      overhangPenalty: over,
      footprint: foot,
      maxCross: cross,
      shadowed,
      estHeight: 1,
      stability: 'stable',
      stabilityMargin: 1,
      contactArea: 1,
      compositeScore: 0,
    };
  }

  it("orders by overhang-only when weights select it", () => {
    const cs = [mkCand(10, 0, 0), mkCand(1, 0, 0), mkCand(5, 0, 0)];
    const ranked = rankByWeights(cs, { wOverhang: 1, wFootprint: 0, wCross: 0 });
    expect(ranked[0].overhangPenalty).toBe(1);
    expect(ranked[1].overhangPenalty).toBe(5);
    expect(ranked[2].overhangPenalty).toBe(10);
  });

  it("orders by cross-only when weights select it", () => {
    const cs = [mkCand(0, 0, 3), mkCand(0, 0, 1), mkCand(0, 0, 2)];
    const ranked = rankByWeights(cs, { wOverhang: 0, wFootprint: 0, wCross: 1 });
    expect(ranked.map(c => c.maxCross)).toEqual([1, 2, 3]);
  });

  it("is pure: input is not mutated", () => {
    const cs = [mkCand(10, 0, 0), mkCand(1, 0, 0)];
    const snapshot = cs.map(c => c.compositeScore);
    rankByWeights(cs, { wOverhang: 1, wFootprint: 0, wCross: 0 });
    expect(cs.map(c => c.compositeScore)).toEqual(snapshot);
  });

  it("returns empty for empty input", () => {
    expect(rankByWeights([], { wOverhang: 1, wFootprint: 1, wCross: 1 })).toEqual([]);
  });
});

describe("rankByConsensus", () => {
  function mkCand(over: number, foot: number, cross: number, shadowed = 0): Candidate {
    return {
      id: `c-${over}-${foot}-${cross}-${shadowed}`,
      quaternion: [1, 0, 0, 0],
      overhangPenalty: over, footprint: foot, maxCross: cross, shadowed,
      estHeight: 1, stability: 'stable', stabilityMargin: 1, contactArea: 1, compositeScore: 0,
    };
  }

  it("favours candidates that are good across all metrics, not just one", () => {
    // A is best on overhang but worst on cross → score = 1-1.0 = 0%
    // B is mid on everything → score = 1-0.444 = 55.6%
    // C is worst on overhang but best on rest → score = 1-1.0 = 0%
    // B wins with highest score.
    const cs = [
      mkCand(1, 10, 10, 0),
      mkCand(5, 5, 5, 0),
      mkCand(10, 1, 1, 0),
    ];
    const ranked = rankByConsensus(cs);
    expect(ranked[0].id).toBe('c-5-5-5-0');
    expect(ranked[0].compositeScore).toBeCloseTo(0.556, 2);
  });

  it("penalises candidates with high shadowed fraction", () => {
    // A: shadowed=0.9 → worst=1.0 → score=0%
    // B: shadowed=0.1 → worst≈0.0 → score≈100%
    const cs = [
      mkCand(5, 5, 5, 0.9),
      mkCand(5, 5, 5, 0.1),
    ];
    const ranked = rankByConsensus(cs);
    expect(ranked[0].id).toBe('c-5-5-5-0.1');
    expect(ranked[0].compositeScore).toBeCloseTo(1, 3);
  });

  it("returns empty for empty input", () => {
    expect(rankByConsensus([])).toEqual([]);
  });

  it("is pure: input not mutated", () => {
    const cs = [mkCand(1, 2, 3), mkCand(4, 5, 6)];
    const snap = cs.map(c => c.compositeScore);
    rankByConsensus(cs);
    expect(cs.map(c => c.compositeScore)).toEqual(snap);
  });
});

describe("shadowedOverhangFraction", () => {
  // Unit square in XY plane: two triangles, normal +Z, total area 1.0.
  function unitSquareData() {
    const normals = new Float32Array([0, 0, 1, 0, 0, 1]);
    const areas = new Float32Array([0.5, 0.5]);
    const positions = new Float32Array([
      0, 0, 0, 1, 0, 0, 1, 1, 0,
      0, 0, 0, 1, 1, 0, 0, 1, 0,
    ]);
    return { normals, areas, positions };
  }

  it("no overhang (face normal points away from dir) returns 0", () => {
    // Dir = -Z, normals are +Z → dot < 0 → no overhang
    const { normals, areas, positions } = unitSquareData();
    expect(shadowedOverhangFraction([0, 0, -1], positions, normals, areas, 30)).toBeCloseTo(0, 5);
  });

  it("lone bottom face is clear (not shadowed)", () => {
    // Dir = +Z: the square IS the bottom surface → clear
    const { normals, areas, positions } = unitSquareData();
    const frac = shadowedOverhangFraction([0, 0, 1], positions, normals, areas, 30, 16, 0.02);
    expect(frac).toBeLessThan(0.05);
  });

  it("detects overhang shadowed by floor below", () => {
    // Floor at z=0 (big), ceiling at z=5 (small, directly above), both face +Z.
    // Dir = +Z: both overhang, ceiling shadowed by floor.
    const normals = new Float32Array([0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1]);
    const areas = new Float32Array([100, 100, 2, 2]);
    const positions = new Float32Array([
      -10, -10, 0,  10, -10, 0,  10, 10, 0,
      -10, -10, 0,  10, 10, 0,   -10, 10, 0,
      -1, -1, 5,  1, -1, 5,  1, 1, 5,
      -1, -1, 5,  1, 1, 5,   -1, 1, 5,
    ]);
    const frac = shadowedOverhangFraction([0, 0, 1], positions, normals, areas, 30, 16, 0.02);
    expect(frac).toBeGreaterThan(0);
    expect(frac).toBeLessThan(0.05);
  });

  it("separate columns do not shadow each other", () => {
    // Two squares at same z=0 but far apart in XY → no shadowing.
    const normals = new Float32Array([0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1]);
    const areas = new Float32Array([0.5, 0.5, 0.5, 0.5]);
    const positions = new Float32Array([
      -11, -1, 0,  -9, -1, 0,  -9, 1, 0,
      -11, -1, 0,  -9, 1, 0,   -11, 1, 0,
       9, -1, 0,  11, -1, 0,  11, 1, 0,
       9, -1, 0,  11, 1, 0,    9, 1, 0,
    ]);
    const frac = shadowedOverhangFraction([0, 0, 1], positions, normals, areas, 30, 16, 0.02);
    expect(frac).toBeLessThan(0.05);
  });

  it("empty mesh returns 0", () => {
    expect(shadowedOverhangFraction([0, 0, 1], new Float32Array(0), new Float32Array(0), new Float32Array(0), 30)).toBe(0);
  });
});
