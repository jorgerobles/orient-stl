---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: active
stopped_at: Phase 3.5 complete — scoring expansion & refinement delivered
last_updated: "2026-07-13T10:15:00.000Z"
last_activity: "2026-07-13 — Phase 3.5 verified: 12/12 success criteria met; 3 rankers, 8 profiles, seeded refine, UI switcher"
progress:
  total_phases: 5
  completed_phases: 4
  total_plans: 11
  completed_plans: 11
  percent: 85
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-07-11)

**Core value:** Generate a reliable orientation ranking that minimizes supports and maximizes print success, without the user manually rotating the model.
**Current focus:** Phase 4 — v3 UX Polish (next)

## Current Position

Phase: 3.5 (scoring-expansion) — ✅ Complete
Plan: 2 of 2 (both complete)
Status: Verified — 12/12 success criteria met
Last activity: 2026-07-13 -- Phase 3.5 verified; profiles, seeded refine, TOPSIS, UI switcher delivered

Progress: [██████████████████] ~85%

### Phase 2 status (final)

| Plan | Status | Notes |
|------|--------|-------|
| 02-01 Viewport | ✅ Complete | Centering fix: centroid baked into geometry via `geometry.translate` |
| 02-02 Yaw | ✅ Complete | Linear slider + 45° snap; circular dial deferred to Phase 3 overlay |
| 02-03 Export | ✅ Complete | Binary STL export with quaternion transform |

### Next step

Phase 4 — v3 UX Polish (Thumbnail Strip, Favorites, ZIP Export)

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
| 4. v3 UX Polish | 0/3 | — | — |
| 3.5 Scoring Expansion & Refinement | 2/2 ✅ | 32 min | 3 tasks + backfill |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- **[Phase 1 drift]** WASM boundary is `prepare_data()` (geometry + directions), NOT `compute_orientations()`. Scoring/stability/yaw moved to JS Web Workers for iteration speed. (01-02-SUMMARY)
- **[Phase 1 drift]** Phase 2 scope (viewport, export, heatmap) landed inside Phase 1's JS work. Phase 2 plans must reconcile, not re-implement. (01-03-SUMMARY)
- **[Phase 2]** Centroid (vertex average) is the rotation pivot, not bbox-center → stable rotation around center of mass
- **[Phase 2]** Full candidate quaternion = `qYaw * qAlign(dir, -Y)` — align candidate dir to -Y first, then apply yaw
- **[Phase 2]** Multi-worker: split candidate directions across `navigator.hardwareConcurrency - 1` workers
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
| Cleanup | `multiplyQuats` duplicated in main.ts and compute.ts | Minor | Phase 2 |

## Session Continuity

Last session: 2026-07-13T08:12:52.340Z
Stopped at: Completed 03.5-02-PLAN.md
Resume file: None

### Infrastructure State

- Vite dev server: running at http://localhost:5173/
- WASM binary: `web/pkg/orient_core_bg.wasm` (132KB)
- TypeScript: `npx tsc --noEmit` passes
- Build: `npm run build` succeeds
