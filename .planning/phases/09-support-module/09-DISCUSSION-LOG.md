# Phase 9: Support Module - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-08-27
**Phase:** 09-support-module
**Areas discussed:** Performance budget, Degenerate mesh handling, Algorithm tuning parameters

---

## Performance Budget

### Compute cost approach

| Option | Description | Selected |
|--------|-------------|----------|
| Rayon parallelism | Use rayon thread pool for island detection + volume classification per-island | |
| Cap mesh size | Reject meshes over 100K triangles with clear error | |
| Progressive quality | Run island detection on coarser grid first (fast preview), then refine | |
| BVH acceleration + coarse grid pass | Build BVH over triangles for ray queries, rasterize at coarser grid (1mm) for island detection, refine to 0.5mm only for contact placement | ✓ |

**User's choice:** BVH acceleration + coarse grid pass (after noting rayon was counterproductive in earlier stages)
**Notes:** User referenced "algorithmia" — likely referring to algorithmic efficiency over parallelism. Rayon doesn't help in WASM and was counterproductive before.

### Progress reporting

| Option | Description | Selected |
|--------|-------------|----------|
| Per-stage progress | Report 4 stages: islands → classification → placement → raft | ✓ |
| Per-layer progress | Report each layer during rasterization | |
| No progress needed | Support generation should be fast enough (<2s) | |

**User's choice:** Per-stage progress (Recommended)
**Notes:** Matches pipeline.ts onProgress pattern.

### Max caps

| Option | Description | Selected |
|--------|-------------|----------|
| Keep HANDOFF caps | 500 islands, 10000 contacts. Excess truncated with warning | ✓ |
| Remove caps entirely | No limits. Risk: pathological meshes could blow up WASM memory | |
| Lower caps | 200 islands, 5000 contacts | |

**User's choice:** Keep HANDOFF caps (Recommended)
**Notes:** Covers all practical models.

---

## Degenerate Mesh Handling

### No-overhang scenario

| Option | Description | Selected |
|--------|-------------|----------|
| Return empty result | SupportResult with supports=[], raft=[], island_count=0 | |
| Return minimal supports | Place a few contacts at the bottom pole anyway | |

**User's choice:** "that's theoretical but not physics real. research or 2"
**Notes:** User pointed out the scenario is theoretical — real meshes always have overhangs. Redirected to realistic degenerate cases.

### Zero-area triangles, non-manifold edges, thin features

| Option | Description | Selected |
|--------|-------------|----------|
| Filter + warn | Skip zero-area triangles during rasterization, emit warning count | ✓ |
| Pre-validate mesh | Run mesh validation before support generation | |
| Agent's discretion | Let the planner decide | |

**User's choice:** Filter + warn (Recommended)
**Notes:** Don't crash, don't fabricate data.

### Density cap for huge overhangs

| Option | Description | Selected |
|--------|-------------|----------|
| Density cap per island | Cap contacts per island at ~200 regardless of volume | ✓ |
| No cap — let volume drive it | Volume classification determines type, Poisson-disk handles density | |
| Agent's discretion | Planner decides based on what's practical | |

**User's choice:** Density cap per island (Recommended)
**Notes:** Large overhangs get wider spacing, not infinitely dense supports.

---

## Algorithm Tuning Parameters

### Volume thresholds

| Option | Description | Selected |
|--------|-------------|----------|
| Lock as defaults, expose in config | HANDOFF values become SupportConfig defaults, Phase 10 UI adds sliders | ✓ |
| Lock permanently | Hardcode the thresholds, no UI override | |
| Leave as open config | No defaults — user must set them in Phase 10 UI | |

**User's choice:** Lock as defaults, expose in config (Recommended)
**Notes:** HANDOFF values (Light <50mm³, Medium 50-500mm³, Heavy >500mm³) are sane defaults.

### Poisson-disk spacing ranges

| Option | Description | Selected |
|--------|-------------|----------|
| Lock HANDOFF ranges | Light 2.5-6mm, Medium 2-5mm, Heavy 1.5-3.5mm as defaults | ✓ |
| Agent's discretion | Planner tunes based on testing | |

**User's choice:** Lock HANDOFF ranges (Recommended)
**Notes:** Standard resin printing spacing values.

### Tip diameters and penetration depths

| Option | Description | Selected |
|--------|-------------|----------|
| Lock all as defaults | All HANDOFF values become SupportConfig defaults | |
| Lock penetration, defer tips | Penetration geometrically constrained, tip diameter is user-preference | ✓ |

**User's choice:** Lock penetration, defer tips
**Notes:** Penetration (0.2/0.3/0.4mm) must be > layer height. Tip diameters (0.25/0.40/0.80mm) deferred to Phase 10 UI.

### Curvature thresholds

| Option | Description | Selected |
|--------|-------------|----------|
| Lock HANDOFF thresholds | cos>0.7 → Heavy, 0.3<cos<0.7 → Medium, cos<0.3 → Light | ✓ |
| Agent's discretion | Planner tunes based on testing | |

**User's choice:** Lock HANDOFF thresholds (Recommended)
**Notes:** Standard curvature classification for cylindrical features.

### Raft dimensions

| Option | Description | Selected |
|--------|-------------|----------|
| Lock as defaults | 1.0mm thickness, 1.5mm line width | ✓ |
| Agent's discretion | Planner decides based on testing | |

**User's choice:** Lock as defaults (Recommended)
**Notes:** Standard raft dimensions for resin printing.

### Grid cell size

| Option | Description | Selected |
|--------|-------------|----------|
| Lock 0.5mm | Standard for resin printing (10x layer height) | ✓ |
| Agent's discretion | Planner tunes based on performance testing | |

**User's choice:** Lock 0.5mm (Recommended)
**Notes:** Good accuracy/speed balance.

---

## the agent's Discretion

- BVH implementation approach (simple bounding-box tree vs `bvh` crate)
- Bowyer-Watson Delaunay implementation details
- Kruskal's MST implementation details
- Whether to expose `default_config()` WASM export
- Rust module file structure

## Deferred Ideas

- Tip diameters (0.25/0.40/0.80mm) → Phase 10 UI as user-preference sliders
- User-editable profile creation UI → future phase
- WebGPU acceleration → aspirational
- WASM multithreading → not applicable (rayon counterproductive in WASM)
