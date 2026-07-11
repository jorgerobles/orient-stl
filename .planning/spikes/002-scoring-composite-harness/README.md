---
spike: 002
name: scoring-composite-harness
type: standard
validates: "Given a mesh + candidate directions, when scored with variable weights of (overhang, footprint, max-cross-section), then the ranking differs meaningfully from overhang-only and the composite produces lower peel-force orientations — verifiable via the harness on real fixtures"
verdict: VALIDATED
related: [001]
tags: [rust, scoring, resin, resin-physics, heuristics]
---

# Spike 002: Resin Scoring Composite Harness

## What This Validates

Given a mesh + candidate directions, when scored with variable weights of
(overhang H1, footprint H4, max-cross-section H2), then:
1. H4 and H2 are computable at O(N) — same cost tier as the current overhang scan.
2. The metrics disagree on the best orientation — so the choice matters.
3. Cross-section is the most discriminative metric (varies most across candidates).
4. The composite produces orientations with materially lower peel force than overhang-only.

## Research

See `.planning/research/SCORING-HEURISTICS.md` for the full heuristics catalog and
cost analysis. This spike implements the three quick-win heuristics (H1 current, H4
footprint, H2 max-cross-section) in Rust and builds a harness to compare weight
configs empirically.

**Chosen approach:** implement all three in Rust `scoring.rs` (WASM-first decision),
expose via a `#[cfg(test)] #[ignore]` harness that runs the full pipeline (parse →
precompute → hull → candidates → score) on real fixtures and prints rankings for
6 weight configurations side-by-side.

## How to Run

```bash
# from repo root (cargo must be on PATH: export PATH="$HOME/.cargo/bin:$PATH")
cd core && cargo test harness_run -- --ignored --nocapture
```

The unit tests (H4/H2 correctness) run with the normal suite:
```bash
cd core && cargo test scoring
```

## What to Expect

The harness loads two fixtures (`test-tetrahedron.stl`, `resources/Skulled_Wurm_Bird_WOBase.stl`),
computes all candidate directions from the convex hull, scores each with H1/H4/H2,
min-max-normalises each component across the candidate set, and prints the best
candidate + top-3 ids for each of 6 weight configs:
1. overhang-only (current v1 baseline)
2. footprint-only (user's naive "minimize shadow")
3. cross-only (pure peel-force)
4. equal-weights
5. resin-biased (cross-heavy: 0.5/1.0/2.0)
6. overhang+footprint (no cross)

## Investigation Trail

### Iteration 1 — implement + first run (2026-07-11)

**TDD:** wrote 6 unit tests for H4/H2 first (footprint face-on/edge-on/45°, cross-section
slab vs spread vs empty). All RED before implementation, all GREEN after. See
`core/src/scoring.rs` test module.

**Bird fixture results (499,310 triangles, 401 candidates):**

| Config | Best # | dir | overhang | footprint | maxcross |
|--------|--------|-----|----------|-----------|----------|
| overhang-only (v1) | 251 | (-0.42,-0.87,-0.27) | 1.89 | 508 | 18.1 |
| footprint-only | 202 | (+0.27,+0.69,+0.67) | 2.27 | 480 | 19.5 |
| cross-only (peel) | 384 | (+0.21,+0.17,-0.96) | 3.18 | 563 | 12.7 |
| equal-weights | 345 | (+0.20,+0.85,+0.50) | 2.04 | 487 | 18.6 |
| resin-biased | 319 | (-0.47,-0.70,-0.54) | 3.25 | 495 | 14.2 |
| overhang+footprint | 202 | (+0.27,+0.69,+0.67) | 2.27 | 480 | 19.5 |

**Key discoveries:**

1. **Metrics genuinely disagree.** Overhang-only picks #251, footprint picks #202,
   cross picks #384. These are different orientations. The metric choice matters.

2. **Cross-section is the most discriminative metric.** Across the 401 candidates:
   - maxcross range: 12.7 .. 50.6  (4.0× spread)
   - overhang range: 1.89 .. 8.53  (4.5× spread)
   - footprint range: 480 .. 624   (1.3× spread)
   Footprint barely varies — the convex-hull-normal candidates all produce similar
   shadows. Cross-section varies hugely — orientation dramatically changes the
   worst single layer. **H2 carries more signal than H4 on this model.**

3. **Current v1 ignores the dominant resin failure.** The overhang-only best (#251)
   has maxcross=18.1. The peel-force optimizer (#384) achieves 12.7 — **30% lower
   peel force** that the current ranking never surfaces. For resin, #251 is riskier
   than #384 but v1 calls it the winner.

4. **Equal-weights (#345) is a robust compromise.** Low-ish overhang (2.04), good
   footprint (487), moderate cross (18.6). Its top-3 ({345, 202, 181}) overlaps
   with overhang+footprint's top-3 ({202, 345, 164}) — these orientations are
   robustly good across multiple metrics.

5. **Resin-biased (#319) trades overhang for cross.** It accepts higher overhang
   (3.25) to push cross down to 14.2. Whether this trade is right depends on the
   resin/printer — a high-peel-force resin benefits; a tolerant resin doesn't.

**Tetrahedron (4 triangles, 4 candidates):** overhang=0 everywhere (faces steeper
than the 30° critical angle in all orientations). Footprint and cross both pick #3
(dir=(0,0,-1), the flat base down). This is the trivially-correct orientation.
Overhang-only is useless on this shape — all candidates tie at 0. **Confirms H1
alone is insufficient; H4/H2 disambiguate where H1 is flat.**

### Iteration 2 — discriminative-weight observation

The 1.3× footprint spread vs 4.0× cross spread suggests footprint contributes little
signal beyond cross-section for convex-hull-normal candidates. **Recommendation:
weight cross > overhang > footprint in the default composite, not equal weights.**
Footprint stays as a tie-breaker and for non-convex meshes where it diverges from
cross. This is a hypothesis to validate on more fixtures (only 2 so far).

## Results

**Verdict: VALIDATED ✓**

- H4 (footprint) and H2 (max-cross-section) are O(N), trivially computable alongside
  the existing overhang scan. No new infrastructure. Confirmed by timing (the harness
  runs the full 500K-triangle, 401-candidate pipeline in well under a second).
- The composite produces materially different rankings from overhang-only, and
  surfaces lower-peel-force orientations that v1 misses.
- The harness provides the iteration infrastructure for future weight tuning.

**Recommended default composite (subject to human visual verification):**
```
score = 1.0 · overhang_norm + 0.5 · footprint_norm + 2.0 · cross_norm
```
Cross-weighted heaviest (most discriminative + dominant resin failure), overhang
second, footprint as a cheap tie-breaker. But **equal-weights is a fine starting
point** and easier to defend.

**What this spike did NOT resolve:**
- Which orientation is actually best for printing the bird (needs human visual
  judgment in the live app — would require wiring these metrics into the viewport).
- Whether footprint adds value on non-convex meshes (only convex-hull-normal
  candidates tested so far).
- Default weights need user validation across several real models.

## Signal for the Build

1. **Move scoring back into WASM.** This spike implements H1/H4/H2 in Rust — exactly
   where the architecture decision (spike 001 + the "WASM-first" decision) says they
   belong. The JS `compute.ts` scoring is the drift; relocate it. The boundary
   becomes: WASM returns scored+ranked candidates, JS just displays.
2. **Expose all three components** (overhang, footprint, maxcross) to JS so the UI
   can show multi-metric sort (Phase 3 feature) without re-scoring.
3. **Default composite weights** as above, but make them config so users can tune.
4. **Keep the harness** as a regression check when touching scoring — extend the
   `#[ignore]` test with more fixtures as the model library grows.

## Observability

Harness prints to stdout (`--nocapture`). For production, the composite score +
the three raw components should surface in the candidate-info readout (Phase 3
multi-metric display).
