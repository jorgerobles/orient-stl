---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: executing
stopped_at: Phase 6 added to roadmap — Frontend Architecture Refactor
last_updated: "2026-07-14T08:12:08.239Z"
last_activity: 2026-07-14 -- Phase 06 execution started
progress:
  total_phases: 7
  completed_phases: 5
  total_plans: 18
  completed_plans: 14
  percent: 71
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-07-11)

**Core value:** Generate a reliable orientation ranking that minimizes supports and maximizes print success, without the user manually rotating the model.
**Current focus:** Phase 06 — frontend-architecture-refactor-state-management-modularizati

## Current Position

Phase: 06 (frontend-architecture-refactor-state-management-modularizati) — EXECUTING
Plan: 1 of 4
Status: Executing Phase 06
Last activity: 2026-07-14 -- Phase 06 execution started

### Phase 5 status (final)

| Plan | Status | Notes |
|------|--------|-------|
| 05-01 Rust ranking + selection + yaw | ✅ Complete | TDD: ground-truth tests for ranking, selection, yaw; Cargo.toml dual-target |
| 05-02 WASM exports + CLI | ✅ Complete | score_all_directions, rank_candidates, select_diverse, compute_norm_bounds; CLI binary; drop self-referential tests |
| 05-03 TS thin layer | ✅ Complete | compute.ts stripped, compute.test.ts deleted, single-worker dispatcher, main.ts updated |
| 05-04 Cross-verification | ✅ Complete | 12 CLI ref outputs, float-layout verification, single Rust source guarantees parity |

### Next step

Plan Phase 6 — Frontend Architecture Refactor

## Performance Metrics

**Velocity:**

- Total plans completed: 6 (Phases 1+2)
- Average duration: —
- Total execution time: —

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 1. Rust WASM Core Engine + Build Toolchain | 3/3 ✅ | — | — |
| 2. Viewport + Yaw + Export | 3/3 ✅ | — | — |
| 3. v2 Enhancements | 2/2 ✅ | — | — |
| 4. v3 UX Polish | 0/3 | [-] Dropped (YAGNI) | 2026-07-14 |
| 6. Frontend Architecture Refactor | 0/0 | Not planned | - |
| 3.5 Scoring Expansion & Refinement | 2/2 ✅ | 32 min | 3 tasks + backfill |

## Accumulated Context

### Roadmap Evolution

- Phase 6 added: Frontend Architecture Refactor — split god module, state management, Viewport modularization, CSS extraction, accessibility

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- **[Phase 1 drift]** WASM boundary is `prepare_data()` (geometry + directions), NOT `compute_orientations()`. Scoring/stability/yaw moved to JS Web Workers for iteration speed. (01-02-SUMMARY)
- **[Phase 1 drift]** Phase 2 scope (viewport, export, heatmap) landed inside Phase 1's JS work. Phase 2 plans must reconcile, not re-implement. (01-03-SUMMARY)
- **[Phase 2]** Centroid (vertex average) is the rotation pivot, not bbox-center → stable rotation around center of mass
- **[Phase 2]** Full candidate quaternion = `qYaw * qAlign(dir, -Y)` — align candidate dir to -Y first, then apply yaw
- ~~**[Phase 2]** Multi-worker: split candidate directions across `navigator.hardwareConcurrency - 1` workers~~ (replaced in Phase 5 with single-worker WASM dispatcher — score_all_directions → rank_candidates → select_diverse runs entirely inside a thin worker)
- **[Phase 2]** Decimate both positions and normals/areas to ~12K elements for 50x scoring speedup
- **[Phase 2]** Consensus ranking (minimax) is the only ranking needed — 100% = best, 0% = worst
- **[Phase 2]** H11 shadowed-overhang uses 8-sample yaw minimisation per direction
- **[Phase 2]** Yaw slider applies rotation only on `change` (release), not `input` (drag) — no ground-snap during drag
- **[Phase 2]** Yaw overlay + score feedback deferred to Phase 3
- **[Phase 2]** Loading progress bar yields to rendering pipeline via `setTimeout(0)` between sync phases
- **[Spike resolved]** stl_io compiles cleanly to wasm32-unknown-unknown — no vendored parser needed. WASM-first for computation; WebGPU is the documented upgrade path for future perf-critical phases.
- **[Phase 3.5]** PrusaSlicer codegraph comparison (Rotfinder.cpp) surfaced 2 missing heuristics: H5 surface-quality (axis-misalignment, Prusa "Best surface quality") + H6 print-height (Prusa "Lowest Z height"). Both added with TS+Rust parity; both rankers now cover all 5 metrics.
- **[Phase 3.5]** `rankByConsensus` rewritten: replaced `overhang × (1 + hN×0.5)` fudge with height as its own cost term in the max(); surfaceQuality inverted to cost form. All five heuristics now have equal veto power.
- **[Phase 3.5]** Research: MCDA literature confirms equal-weight minimax is brittle (dictatorial veto); configurable per-metric weights + TOPSIS are the textbook fixes. Follow-on work: externalise profiles to JSON, add TOPSIS ranker, seed refine determinism + variance metric.
- **[Phase 3.5 P02]** TOPSIS MCDA uses vector normalisation + Euclidean distance to ideal; closeness [0,1]
- **[Phase 3.5 P02]** Weight profiles externalised to JSON via import.meta.glob (build-time, no runtime fetch)
- **[Phase 3.5 P02]** RefineFn callback injection enables pipeline testing without WASM dependency
- **[Phase 3.5 P02]** xorshift32 PRNG eliminates click-to-click non-determinism in Varita Mágica refine
- **[Phase 3.5 P02]** K=4 batch refine with refinedOverhang (min) + refineVariance (stddev) per candidate
- **[Post-3.5]** Varita button removed (redundant — refinement already runs automatically in computeSlice via WASM batch refine)
- **[Post-3.5]** All 3 rankers (`rankByWeights`, `rankByConsensus`, `rankByTopsis`) now use `refinedOverhang` instead of raw `overhangPenalty` — refinement results actually affect ranking
- **[Post-3.5]** `mergeCandidates` sorts by `refinedOverhang` instead of raw `penalty` for better initial diversity selection
- **[Post-3.5]** Profile-aware `mergeCandidates`: when weights are provided, sorts by weighted composite (min-max normalised) so diversity selection reflects the chosen profile
- **[Post-3.5]** "Recalculate" button at bottom of config panel; enabled on dirty (profile/angle change), disabled on clean; triggers full recompute with current profile weights
- **[Post-3.5]** Overlay live score now computes actual direction metrics (overhang/footprint/cross/surface/height) via consensus formula instead of nearest-candidate lookup — shows real score for all 5 axes

### Pending Todos

None active.

### Blockers/Concerns

None.

## Deferred Items

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Architecture | Move scoring/stability/yaw back into WASM if JS proves too slow | Deferred (v2/v3) | Phase 1 |
| Performance | WebGPU compute pipeline for GPU-class throughput | Aspirational (v2/v3) | Spike |
| Phase 3 | 3D manipulation overlay (yaw/tilt with score feedback) | Phase 3 | Phase 2 close |
| Phase 3 | Circular yaw dial + geometry snap | Phase 3 | Phase 2 close |
| Cleanup | ~~`multiplyQuats` duplicated in main.ts and compute.ts~~ | Resolved — compute.ts stripped | Phase 5 |

## Session Continuity

Last session: 2026-07-14T12:00:00.000Z
Stopped at: Phase 6 added to roadmap — Frontend Architecture Refactor
Resume file: None

### Infrastructure State

- Vite dev server: stopped
- WASM binary: `web/pkg/orient_core_bg.wasm` (195KB — all metrics + ranking + selection)
- TypeScript: `npx tsc --noEmit` passes (38 tests)
- Build: `npm run build` succeeds
- Rust: `cargo test` passes (78 unit + 1 integration)
- TS metric/ranking/selection functions: **0 remaining** (all migrated to Rust WASM)
