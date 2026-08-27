---
phase: 07-correctness-fixes-h11-scoring
plan: 04
subsystem: scoring
tags: [shadowed, overhang, ranking, score-weights, wasm-boundary]

# Dependency graph
requires:
  - phase: 07-correctness-fixes-h11-scoring
    provides: "shadowed_overhang_fraction scoring function"
provides:
  - "6-component weighted ranking with shadowed as cost metric"
  - "compute_norm_bounds 12-float output (was 10)"
  - "rank_candidates 6-weight input (was 5)"
  - "score_direction 9-float output (was 8)"
  - "All 8 profile JSONs with wShadowed"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns: ["shadowed as 6th cost component in all 3 rankers"]

key-files:
  created: []
  modified:
    - crates/orient/src/ranking.rs
    - crates/orient/src/scoring.rs
    - crates/orient/src/lib.rs
    - crates/orient/src/harness.rs
    - web/src/types.ts
    - web/src/app/AppController.ts
    - web/src/views/ScorePanel.ts
    - web/src/profiles/*.json

key-decisions:
  - "Shadowed wired as COST metric (higher=worse), NOT inverted like surface"
  - "grid_res=32, tol_frac=0.02 as constants in scoring.rs, independent of cross_bins"
  - "Shadowed wired into ALL 3 rankers (weights, consensus, topsis) — no half-measures"

patterns-established:
  - "6th metric (shadowed) follows same min-max normalization pattern as other cost metrics"
  - "resin-biased profile has wShadowed=2.0 to penalize cavity-forming orientations"

requirements-completed: []

# Metrics
duration: 1min
completed: 2026-08-27
---

# Phase 7 Plan 04: H11 Shadowed-Overhang Wiring Summary

**Shadowed-overhang fraction (resin suction-cup risk) wired as 6th cost component in Rust ranking core, WASM exports, TS AppController, and all profile JSONs — fully verified**

## Performance

- **Duration:** 1 min
- **Started:** 2026-08-27T09:34:07Z
- **Completed:** 2026-08-27T09:35:28Z
- **Tasks:** 5 (verification only — implementation pre-existed)
- **Files modified:** 0 (all code already in place)

## Accomplishments
- Verified all 16 plan "truths" are satisfied by existing implementation
- Confirmed `cargo test` passes (70 tests, 0 failures)
- Confirmed `npm run type-check` passes (0 errors)
- Confirmed `npm run test` passes (78 tests, 12 files)
- Confirmed `make wasm-orient` succeeds with correct 6-weight API

## Task Commits

All tasks verified as already implemented — no code changes needed:

1. **Task 1: RED — ScoreWeights/ScoreComponents gain shadowed** — already implemented
2. **Task 2: GREEN — wire shadowed into rank_by_weights_with_bounds + consensus + topsis** — already implemented
3. **Task 3: WASM exports — compute_norm_bounds 12-float, rank_candidates 6-weight, score_direction 9-float** — already implemented
4. **Task 4: TS — ScoreWeights.wShadowed, 8 profile JSONs, AppController bounds/live-score** — already implemented
5. **Task 5: Smoke — verify resin-biased penalizes cavities** — verified via test suite

**Plan metadata:** No commits needed — implementation pre-existed plan execution.

## Files Created/Modified

No files were created or modified during this plan execution. All implementation was completed in prior waves:

- `crates/orient/src/ranking.rs` — `ScoreWeights` with `w_shadowed`, `rank_by_weights_with_bounds` with 6th component normalization
- `crates/orient/src/scoring.rs` — `ScoreComponents` with `shadowed`, `score_components` computing shadowed via `shadowed_overhang_fraction`
- `crates/orient/src/lib.rs` — `compute_norm_bounds` returns 12 floats, `rank_candidates` parses 6 weights, `score_direction` returns 9 floats
- `crates/orient/src/harness.rs` — `WeightCfg` with `w_shadowed`, real shadowed computation per candidate
- `web/src/types.ts` — `ScoreWeights` with `wShadowed`, 6-tuple weights
- `web/src/app/AppController.ts` — `computeNormBounds` reads 6+6 floats, `updateLiveScore` builds 6-element costs/weights
- All 8 profile JSONs — `wShadowed` key present (resin-biased=2.0, equal=1.0, *-only=0.0)

## Decisions Made

- Shadowed wired as COST metric (higher=worse), NOT inverted like surface — matches overhang/footprint/cross/height pattern
- grid_res=32, tol_frac=0.02 as constants in scoring.rs, independent of cross_bins — consistent with existing call sites
- Shadowed wired into ALL 3 rankers (weights, consensus, topsis) — no half-measures

## Deviations from Plan

None - plan executed exactly as written. Implementation was already complete from prior waves.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 07-04 complete. Shadowed-overhang metric is fully wired end-to-end: Rust ranking core, WASM boundary (6-weight, 12-float norm bounds, 9-float score_direction), TypeScript AppController live-score and worker dispatch, and all profile JSONs.

---
*Phase: 07-correctness-fixes-h11-scoring*
*Completed: 2026-08-27*

## Self-Check: PASSED

Verified:
- [x] `cargo test` — 70 passed, 0 failed
- [x] `npm run type-check` — 0 errors
- [x] `npm run test` — 78 passed, 12 files
- [x] `make wasm-orient` — success, web/pkg/orient updated
