import { describe, it, expect } from "vitest";
import { footprintArea, maxCrossSection, shadowedOverhangFraction, misalignmentScore, rankByWeights, rankByConsensus, rankByTopsis } from "./compute";
import type { Candidate } from "./compute";
import { loadProfiles } from "./profiles";

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

describe("misalignmentScore", () => {
  it("is at its minimum (area-only) when the face normal aligns with the build direction", () => {
    // Unit square, normal +Z. dir = +Z: |n·dn|=1, |n·e1|=|n·e2|=0 → L1 norm = 1.
    // Score = sum(area × 1) = total area = 1.0. This is the lower bound.
    const { normals, areas } = unitSquareData();
    expect(misalignmentScore([0, 0, 1], normals, areas)).toBeCloseTo(1.0, 5);
  });

  it("increases when the face normal spreads across the orientation axes", () => {
    // Same face, dir = diagonal: |n·dn| = 1/√3, |n·e1| + |n·e2| > 0 → L1 norm > 1.
    // Higher score = more "misaligned" from any single axis (PrusaSlicer rewards this).
    const { normals, areas } = unitSquareData();
    const aligned = misalignmentScore([0, 0, 1], normals, areas);
    const diagonal = misalignmentScore([1, 1, 1], normals, areas);
    expect(diagonal).toBeGreaterThan(aligned);
  });

  it("returns 0 on empty mesh", () => {
    expect(misalignmentScore([0, 0, 1], new Float32Array(0), new Float32Array(0))).toBe(0);
  });

  it("returns 0 for a degenerate (zero) direction", () => {
    const { normals, areas } = unitSquareData();
    expect(misalignmentScore([0, 0, 0], normals, areas)).toBe(0);
  });
});

describe("rankByWeights", () => {
  function mkCand(over: number, foot: number, cross: number, shadowed = 0, surface = 1, height = 1): Candidate {
    return {
      id: `c-${over}-${foot}-${cross}-${shadowed}-${surface}-${height}`,
      quaternion: [1, 0, 0, 0],
      overhangPenalty: over,
      footprint: foot,
      maxCross: cross,
      shadowed,
      surfaceQuality: surface,
      estHeight: height,
      stability: 'stable',
      stabilityMargin: 1,
      contactArea: 1,
      compositeScore: 0,
    };
  }

  it("orders by overhang-only when weights select it", () => {
    const cs = [mkCand(10, 0, 0), mkCand(1, 0, 0), mkCand(5, 0, 0)];
    const ranked = rankByWeights(cs, { wOverhang: 1, wFootprint: 0, wCross: 0, wSurface: 0, wHeight: 0 });
    expect(ranked[0].overhangPenalty).toBe(1);
    expect(ranked[1].overhangPenalty).toBe(5);
    expect(ranked[2].overhangPenalty).toBe(10);
  });

  it("orders by cross-only when weights select it", () => {
    const cs = [mkCand(0, 0, 3), mkCand(0, 0, 1), mkCand(0, 0, 2)];
    const ranked = rankByWeights(cs, { wOverhang: 0, wFootprint: 0, wCross: 1, wSurface: 0, wHeight: 0 });
    expect(ranked.map(c => c.maxCross)).toEqual([1, 2, 3]);
  });

  it("orders by height (lower = better) when wHeight selects it", () => {
    const cs = [mkCand(0, 0, 0, 0, 1, 30), mkCand(0, 0, 0, 0, 1, 10), mkCand(0, 0, 0, 0, 1, 20)];
    const ranked = rankByWeights(cs, { wOverhang: 0, wFootprint: 0, wCross: 0, wSurface: 0, wHeight: 1 });
    expect(ranked.map(c => c.estHeight)).toEqual([10, 20, 30]);
  });

  it("orders by surface (higher = better) when wSurface selects it", () => {
    const cs = [mkCand(0, 0, 0, 0, 1.0), mkCand(0, 0, 0, 0, 1.7), mkCand(0, 0, 0, 0, 1.3)];
    const ranked = rankByWeights(cs, { wOverhang: 0, wFootprint: 0, wCross: 0, wSurface: 1, wHeight: 0 });
    expect(ranked.map(c => c.surfaceQuality)).toEqual([1.7, 1.3, 1.0]);
  });

  it("is pure: input is not mutated", () => {
    const cs = [mkCand(10, 0, 0), mkCand(1, 0, 0)];
    const snapshot = cs.map(c => c.compositeScore);
    rankByWeights(cs, { wOverhang: 1, wFootprint: 0, wCross: 0, wSurface: 0, wHeight: 0 });
    expect(cs.map(c => c.compositeScore)).toEqual(snapshot);
  });

  it("returns empty for empty input", () => {
    expect(rankByWeights([], { wOverhang: 1, wFootprint: 1, wCross: 1, wSurface: 1, wHeight: 1 })).toEqual([]);
  });
});

describe("rankByConsensus", () => {
  function mkCand(over: number, foot: number, cross: number, shadowed = 0, surface = 1, height = 1): Candidate {
    return {
      id: `c-${over}-${foot}-${cross}-${shadowed}-${surface}-${height}`,
      quaternion: [1, 0, 0, 0],
      overhangPenalty: over, footprint: foot, maxCross: cross, shadowed,
      surfaceQuality: surface, estHeight: height,
      stability: 'stable', stabilityMargin: 1, contactArea: 1, compositeScore: 0,
    };
  }

  it("favours candidates that are good across all metrics, not just one", () => {
    // A is best on overhang but worst on cross → worst-normalised = 1.0 → score = 0%
    // B is mid on everything → worst-normalised = 0.444 → score = 55.6%
    // C is worst on overhang but best on rest → worst-normalised = 1.0 → score = 0%
    // B wins with highest score. (surface/height constant → don't break the tie.)
    const cs = [
      mkCand(1, 10, 10, 0),
      mkCand(5, 5, 5, 0),
      mkCand(10, 1, 1, 0),
    ];
    const ranked = rankByConsensus(cs);
    expect(ranked[0].id).toBe('c-5-5-5-0-1-1');
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
    expect(ranked[0].id).toBe('c-5-5-5-0.1-1-1');
    expect(ranked[0].compositeScore).toBeCloseTo(1, 3);
  });

  it("penalises candidates with tall estHeight (lower = better)", () => {
    // Two candidates identical except height. Shorter wins.
    const cs = [
      mkCand(5, 5, 5, 0.1, 1, 50),
      mkCand(5, 5, 5, 0.1, 1, 10),
    ];
    const ranked = rankByConsensus(cs);
    expect(ranked[0].estHeight).toBe(10);
  });

  it("rewards candidates with high surfaceQuality (higher = better)", () => {
    // Two candidates identical except surfaceQuality. Higher wins.
    const cs = [
      mkCand(5, 5, 5, 0.1, 1.0, 1),
      mkCand(5, 5, 5, 0.1, 1.7, 1),
    ];
    const ranked = rankByConsensus(cs);
    expect(ranked[0].surfaceQuality).toBe(1.7);
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

describe("loadProfiles", () => {
  it("returns an object with 8 profile entries", () => {
    const profiles = loadProfiles();
    expect(Object.keys(profiles).length).toBe(8);
  });

  it("has an 'overhang-only' key with wOverhang=1 and all others 0", () => {
    const profiles = loadProfiles();
    expect(profiles["overhang-only"]).toEqual({ wOverhang: 1, wFootprint: 0, wCross: 0, wSurface: 0, wHeight: 0 });
  });

  it("has a 'resin-biased' key with the correct weight values", () => {
    const profiles = loadProfiles();
    expect(profiles["resin-biased"]).toEqual({ wOverhang: 0.5, wFootprint: 1, wCross: 2, wSurface: 0.5, wHeight: 0.5 });
  });

  it("has an 'equal' key where all weights are 1", () => {
    const profiles = loadProfiles();
    expect(profiles["equal"]).toEqual({ wOverhang: 1, wFootprint: 1, wCross: 1, wSurface: 1, wHeight: 1 });
  });

  it("every profile has exactly 5 numeric fields", () => {
    const profiles = loadProfiles();
    for (const [name, w] of Object.entries(profiles)) {
      const keys = Object.keys(w);
      expect(keys.length).toBe(5);
      for (const k of keys) expect(typeof w[k as keyof typeof w]).toBe("number");
    }
  });
});

describe("rankByTopsis", () => {
  function mkCand(over: number, foot: number, cross: number, shadowed = 0, surface = 1, height = 1): Candidate {
    return {
      id: `c-${over}-${foot}-${cross}-${shadowed}-${surface}-${height}`,
      quaternion: [1, 0, 0, 0],
      overhangPenalty: over, footprint: foot, maxCross: cross, shadowed,
      surfaceQuality: surface, estHeight: height,
      stability: 'stable', stabilityMargin: 1, contactArea: 1, compositeScore: 0,
    };
  }

  it("returns empty array for empty input", () => {
    expect(rankByTopsis([], { wOverhang: 1, wFootprint: 1, wCross: 1, wSurface: 1, wHeight: 1 })).toEqual([]);
  });

  it("ranks the best-on-all candidate first", () => {
    // Candidate A: low costs + high surface → closest to ideal-best
    // Candidate B: high costs + low surface → closest to ideal-worst
    const cs = [
      mkCand(10, 10, 10, 0.1, 0.5, 50),
      mkCand(1, 1, 1, 0.9, 1.7, 10),
    ];
    const ranked = rankByTopsis(cs, { wOverhang: 1, wFootprint: 1, wCross: 1, wSurface: 1, wHeight: 1 });
    expect(ranked[0].id).toBe('c-1-1-1-0.9-1.7-10');
  });

  it("treats surfaceQuality as a benefit metric (higher = better) when wSurface > 0", () => {
    const cs = [
      mkCand(0, 0, 0, 0, 1.0, 1),
      mkCand(0, 0, 0, 0, 1.7, 1),
    ];
    const ranked = rankByTopsis(cs, { wOverhang: 0, wFootprint: 0, wCross: 0, wSurface: 1, wHeight: 0 });
    expect(ranked[0].surfaceQuality).toBe(1.7);
  });

  it("treats overhangPenalty as a cost metric (lower = better) when wOverhang > 0", () => {
    const cs = [
      mkCand(10, 0, 0, 0, 1, 1),
      mkCand(1, 0, 0, 0, 1, 1),
    ];
    const ranked = rankByTopsis(cs, { wOverhang: 1, wFootprint: 0, wCross: 0, wSurface: 0, wHeight: 0 });
    expect(ranked[0].overhangPenalty).toBe(1);
  });

  it("produces closeness coefficients in [0, 1]", () => {
    const cs = [
      mkCand(10, 10, 10, 0.9, 0.5, 50),
      mkCand(1, 1, 1, 0.1, 1.7, 10),
      mkCand(5, 5, 5, 0.5, 1.0, 20),
    ];
    const ranked = rankByTopsis(cs, { wOverhang: 1, wFootprint: 1, wCross: 1, wSurface: 1, wHeight: 1 });
    for (const c of ranked) {
      expect(c.compositeScore).toBeGreaterThanOrEqual(0);
      expect(c.compositeScore).toBeLessThanOrEqual(1);
    }
  });

  it("returns closeness 1.0 for a single candidate", () => {
    const cs = [mkCand(5, 5, 5, 0.5, 1.0, 20)];
    const ranked = rankByTopsis(cs, { wOverhang: 1, wFootprint: 1, wCross: 1, wSurface: 1, wHeight: 1 });
    expect(ranked[0].compositeScore).toBeCloseTo(1.0, 5);
  });

  it("is pure: input not mutated", () => {
    const cs = [mkCand(10, 0, 0), mkCand(1, 0, 0)];
    const snap = cs.map(c => c.compositeScore);
    rankByTopsis(cs, { wOverhang: 1, wFootprint: 0, wCross: 0, wSurface: 0, wHeight: 0 });
    expect(cs.map(c => c.compositeScore)).toEqual(snap);
  });
});
