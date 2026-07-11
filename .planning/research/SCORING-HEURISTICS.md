# Research: Resin Printing Orientation Heuristics

**Date:** 2026-07-11
**Trigger:** User question — is "minimize shadow on floor" the right naive heuristic for resin?
**Status:** Synthesized (web sources mostly paywalled/404; foundation validated via Wikipedia SLA article + domain knowledge)

## Physics foundation (why each heuristic matters)

Resin (SLA/DLP/LCD) bottom-up printing works like this (confirmed by Wikipedia
Stereolithography article, "Technology" section):

1. Build plate descends into a vat of liquid photopolymer, touching the transparent
   FEP film at the bottom.
2. UV/light cures one layer cross-section through the FEP.
3. The vat "rocks" / the plate **lifts** to peel the cured layer off the FEP — this is
   the **peel stroke**. The cured layer detaches from the FEP and stays on the plate.
4. Plate descends one layer, fresh resin flows in, repeat.

**Dominant failure modes** (in rough order of frequency for orientation-driven failures):

| Failure | Physics | What causes it |
|---------|---------|----------------|
| Layer separation / pull-off | Peel force > support adhesion | Large cross-section per layer |
| Suction cup | Vacuum trapped in enclosed downward cavity | Concave pockets facing the FEP |
| Island detachment | Small floating feature not anchored | Disconnected regions in a layer |
| Support scarring on visible face | Supports touch cosmetic surface | Orientation exposes key faces down |
| Tip deflection / wobble | Long thin feature cantilevered | Tall features with small anchor |
| Gravity sag (minor in resin) | Self-weight during peel | Large overhangs (less critical than FDM) |

**Key contrast with FDM:** gravity sag is the dominant FDM concern; in resin the dominant
concern is **peel force**, which scales with **cross-section area per layer**. Gravity still
matters but is secondary.

## Heuristics catalog

Each heuristic below: what it measures, why it matters, the math needed, and a cost tier.

Cost tier key (N = triangles in decimated mesh ≈ 12K, D = candidate directions ≈ 200–400):
- **Tier C (Cheap):** O(N) per candidate → O(N·D) total. Single pass over triangles.
- **Tier M (Medium):** O(N log N) per candidate. Sort + sweep, or hull construction.
- **Tier E (Expensive):** O(N·K) per candidate where K = slice count (≈50–100). Requires
  repeated plane-mesh intersection.

---

### H1. Overhang area (area-weighted) — **CURRENT**

- **Measures:** sum of triangle area where `dot(faceNormal, downDir) < -cos(criticalAngle)`.
- **Why:** proxy for support-needed surface and gravity-driven sag.
- **Math per candidate:** for each triangle: 1 dot product + 1 compare + 1 area accumulate.
  ~5 flops/triangle.
- **Cost:** **Tier C** — O(N). Already implemented in `scoring.rs` / `compute.ts:scoreCandidate`.
- **Resin fidelity:** MEDIUM. Captures "faces needing supports" but ignores peel forces
  (the dominant resin failure). Inherited from FDM thinking.

### H2. Max cross-section area (peel-force proxy) — **KEY RESIN METRIC**

- **Measures:** the largest XY cross-section at any Z height along the build direction.
- **Why:** peel force ∝ cross-section area. A single huge layer can rip the print off the
  supports even if everything else looks fine. This is the single most resin-specific metric.
- **Math per candidate:**
  1. Rotate mesh by candidate quaternion.
  2. Slice the mesh at K Z-planes (K ≈ 50–100).
  3. For each slice: intersect triangles with the plane, sum segment lengths → polygon
     areas, or rasterize to a grid and count covered cells.
  - Exact polygon area: O(N·K) with plane-triangle intersection.
  - **Approximation (cheap):** bin triangles into K Z-buckets by centroid Z, sum each
    bucket's projected XY area. O(N) per candidate, O(N·D) total. Overcounts overlaps but
    correlates strongly with true max cross-section.
- **Cost:** **Tier C (approx)** / **Tier E (exact)**. The Z-histogram approximation is a
  quick win — same O(N) as overhang.

### H3. Cross-section gradient (sudden growth)

- **Measures:** max positive derivative of cross-section area vs Z (layer-to-layer growth).
- **Why:** a layer much bigger than the one below it must be peeled against a smaller
  anchor → high rip risk. Gradual growth is safe; sudden growth is dangerous.
- **Math per candidate:** compute H2 per slice, then `max(area[k] - area[k-1])`.
- **Cost:** **Tier C (approx, reuses H2's histogram)** / **Tier E (exact)**.

### H4. Footprint / shadow area on build plate

- **Measures:** area of the mesh's projection onto the XY plane (the "shadow").
- **Why:** proxy for max cross-section (for convex shapes, shadow ≈ max slice). Also
  correlates with base stability and resin-flow restriction during peel.
- **Math per candidate:**
  - Sum of `|projected triangle area|` per triangle: 1 cross product in XY. O(N).
    Overcounts overlapping projections but cheap and monotonic for convex hull normals.
  - Exact (convex hull of projected vertices + hull area): O(N log N) per candidate.
- **Cost:** **Tier C (area-sum approx)** / **Tier M (exact hull)**.
- **Note:** This is the user's "naive heuristic." It is a valid proxy for H2 on convex
  shapes and is extremely cheap.

### H5. Stability (CoM inside contact footprint) — **CURRENT**

- **Measures:** does the center of mass project inside the convex hull of the lowest
  contact points? Binary stable/unstable.
- **Why:** unstable orientations detach during peel.
- **Math per candidate:** O(N) for CoM + O(N log N) for contact-point hull + point-in-polygon.
- **Cost:** **Tier M** — already implemented in `stability.rs` / `compute.ts:checkStability`.

### H6. Print height (Z extent)

- **Measures:** `max(Y) - min(Y)` of the rotated mesh.
- **Why:** taller = more layers = more peel cycles = more cumulative risk + time.
  Also: tall thin features wobble.
- **Math per candidate:** O(N) to find rotated Y extents (or O(1) from a rotated bbox).
- **Cost:** **Tier C** — trivial. Already computed as `estHeight` in `compute.ts:computeHeight`.

### H7. Suction-cup detection

- **Measures:** enclosed downward-facing cavities (concave pockets open downward).
- **Why:** creates vacuum against the FEP → massive peel resistance → catastrophic failure.
- **Math per candidate:** region-growing over the triangle adjacency graph to find
  connected overhang faces forming a closed boundary with an opening facing down.
  Requires adjacency structure. O(N) once adjacency is built, but adjacency build is O(N log N)
  and orientation-dependent.
- **Cost:** **Tier M–E**. Nontrivial to implement correctly. High value but not a quick win.

### H8. Support volume estimate

- **Measures:** total volume of support material needed (downward faces projected to the
  nearest stable surface below).
- **Why:** more support = more material, longer print, more scarring.
- **Math per candidate:** for each downward-facing triangle, ray-cast downward to find
  the first stable triangle; integrate support volume. O(N²) naive, O(N log N) with a
  spatial acceleration structure (BVH/grid).
- **Cost:** **Tier E**. Requires a BVH. Expensive and complex. Defer.

### H9. Island detection

- **Measures:** disconnected regions within a single layer slice (features floating with
  no anchor to the layer below).
- **Why:** islands need their own supports; undetected islands fail.
- **Math per candidate:** slice at K planes + connected-components labeling per slice.
  O(N·K) + union-find.
- **Cost:** **Tier E**. Defer.

### H10. Cosmetic-face protection

- **Measures:** does the orientation put user-marked "important" faces in the down/support
  direction?
- **Why:** supports scar surfaces; orient to keep cosmetic faces up or sideways.
- **Math per candidate:** O(N) over marked faces: dot product vs downDir.
- **Cost:** **Tier C** — once the user selects faces. Requires a face-selection UI
  (out of scope for v1 auto-ranking).

---

## Cost summary table

| ID | Heuristic | Tier | Per-candidate cost | Implemented? | Resin relevance |
|----|-----------|------|--------------------|--------------|-----------------|
| H1 | Overhang area (area-weighted) | C | O(N), ~5 flops/tri | ✅ now | Medium (FDM-flavored) |
| H2 | Max cross-section area | C (approx) / E (exact) | O(N) Z-histogram | ❌ | **High — dominant** |
| H3 | Cross-section gradient | C (reuses H2) | O(N) + O(K) | ❌ | High |
| H4 | Footprint / shadow area | C (approx) / M (exact) | O(N) area-sum | ❌ | High (H2 proxy on convex) |
| H5 | Stability | M | O(N log N) | ✅ now | High |
| H6 | Print height | C | O(N) | ✅ now | Medium |
| H7 | Suction-cup detection | M–E | O(N log N) + region grow | ❌ | High (catastrophic) |
| H8 | Support volume | E | O(N log N) w/ BVH | ❌ | Medium |
| H9 | Island detection | E | O(N·K) + union-find | ❌ | Medium |
| H10 | Cosmetic-face protection | C | O(N) on marked faces | ❌ (needs UI) | User-dependent |

## Quick wins (high value / low cost / not yet implemented)

Ranked by value-to-cost ratio for resin fidelity:

### 🥇 QW1: Footprint area (H4, area-sum approximation) — half a day
- **Why first:** cheapest possible resin-relevant metric. One cross product in XY per
  triangle, sum, done. O(N) per candidate, identical cost shape to the existing overhang
  scan — drops into the same loop.
- **Resin fidelity:** strong proxy for max cross-section on convex hulls (which is exactly
  what the candidate generator samples from). This is the user's "minimize shadow"
  intuition and it is correct for the convex case.
- **Implementation:** in `compute.ts:scoreCandidate`, add `footprint += |cross2D(tri_xy)|`.
  New field on `Candidate`. Sort/rank option.
- **Risk:** overcounts overlapping projections for non-convex meshes. Acceptable for a
  proxy; the ranking still moves the right way.

### 🥈 QW2: Max cross-section (H2, Z-histogram approximation) — 1 day
- **Why second:** the single most resin-faithful metric. The Z-histogram approximation is
  the same cost tier as overhang: bin each triangle by its rotated centroid Z into K bins,
  sum projected XY area per bin, take `max(bin area)`.
- **Resin fidelity:** directly approximates peel force. Beats H4 because it catches the
  worst single layer, not just the overall envelope.
- **Implementation:** K=64 bins, one pass over triangles, track running max. Reuses the
  same rotated-centroid loop as H1.
- **Risk:** bin discretization (K=64 is plenty for ranking).

### 🥉 QW3: Cross-section gradient (H3, reusing H2) — half a day, after QW2
- **Why third:** once H2's histogram exists, H3 is a `max(diff)` over the bins — almost free.
- **Resin fidelity:** catches the "sudden big layer" failure that H2's max alone can miss
  (a model with one huge layer mid-print).

### QW4: Height-weighted composite (combine H1 + H6) — half day
- **Why:** already partially specced as Phase 3 "height-weighted scoring." Penalize tall
  unstable orientations. Cheap.
- **Note:** this was already planned; the research just confirms it's a quick win.

## Defer (high cost or low marginal value)

- **H7 suction-cup** — high value but complex (adjacency + region growing). Worth a
  dedicated Phase 3 plan, not a quick win.
- **H8 support volume** — needs a BVH. Big lift, deferred.
- **H9 island detection** — needs slicing + union-find. Big lift, deferred.
- **H10 cosmetic faces** — needs a face-selection UI first. Phase 4 territory.

## Recommendation for v1 / Phase 2 scoring rework

The current v1 score is pure H1 (overhang area). For resin, the **minimum credible
scoring** should be a weighted composite of:

```
score = w1·overhangArea        // H1, current
      + w2·footprintArea       // H4, QW1 — user's "shadow" intuition, cheap
      + w3·maxCrossSection     // H2, QW2 — dominant resin failure
      + stability (binary reject)  // H5, current
```

H2 and H4 are both O(N) per candidate — the same cost tier as what runs today. Adding
them roughly doubles the per-triangle work (a few extra flops) but does not change the
complexity or the worker architecture. **No new infrastructure needed.**

This moves the app from "FDM-flavored overhang ranker" to "resin-aware ranker" with
roughly a day of work (QW1 + QW2). H3 (gradient) is a natural follow-on once H2 exists.

## Open questions for the user

1. Default weights for w1/w2/w3? Suggested starting point: equal weights, normalized to
   [0,1] across the candidate set, then let the user re-sort by any single metric
   (already a Phase 3 feature — multi-metric sort).
2. Should footprint (H4) use the area-sum approximation or the exact convex-hull-of-
   projection? Area-sum is cheaper and monotonic; exact is more correct for non-convex
   meshes with deep concavities.
3. Is the critical-angle concept (inherited from FDM) still useful for resin, or should
   we drop it in favor of pure cross-section metrics? (Recommendation: keep it — it still
   identifies support-needed faces, just shouldn't be the sole signal.)

## Sources

- Wikipedia, *Stereolithography* — confirms bottom-up SLA physics, peel/vat-rocking,
  support requirement. https://en.wikipedia.org/wiki/Stereolithography
- Slicer documentation (PrusaSlicer, ChiTuBox, Lychee, PreForm) — pages were
  unreachable (403/404/500) during this research. Heuristic list cross-checked against
  known slicer auto-orientation behavior: ChiTuBox and Lychee auto-orient primarily by
  tilt + island detection; PreForm optimizes for support volume + peel. Could not quote
  primary sources directly.
- Domain reasoning from SLA failure physics (peel force ∝ cross-section is standard
  resin-printing knowledge; see Formlabs "Ultimate Guide to SLA" referenced in the
  Wikipedia article).
