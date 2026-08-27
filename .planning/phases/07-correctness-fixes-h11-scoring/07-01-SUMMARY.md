---
phase: 07-correctness-fixes-h11-scoring
plan: 01
subsystem: scoring
tags: [rust, wasM, perpendicularity, hill-climb, correctness]

# Dependency graph
requires:
  - phase: 03.5-scoring-expansion
    provides: perpendicular_basis function in scoring.rs, refine_once hill-climb in lib.rs
provides:
  - tangent_perturbation helper for perpendicular perturbation vectors
  - pub(crate) perpendicular_basis for reuse across modules
  - Corrected refine_once using reusable perpendicular basis
affects: [07-02, 07-03, 07-04, 07-05]

# Tech tracking
tech-stack:
  added: []
  patterns: [perpendicular-basis-reuse, tangent-perturbation-helper]

key-files:
  created: []
  modified:
    - crates/orient/src/lib.rs
    - crates/orient/src/scoring.rs

key-decisions:
  - "Reused existing perpendicular_basis from scoring.rs instead of ad-hoc cross product"
  - "Extracted tangent_perturbation as a pure helper for testability"

patterns-established:
  - "Perpendicular basis pattern: pick non-parallel ref, cross(dir, ref)→e1, cross(dir, e1)→e2"

requirements-completed: []

# Metrics
duration: 0min (already implemented)
completed: 2026-08-27
---

# Phase 7 Plan 01: Tangent Perturbation Correctness Fix Summary

**Replaced ad-hoc non-perpendicular perturbation in refine_once with perpendicular_basis-derived tangent_perturbation helper, eliminating spurious radial component from hill-climb search**

## Performance

- **Duration:** 0 min (already implemented in consolidated Phase 7 commit)
- **Started:** 2026-08-27T09:11:06Z
- **Completed:** 2026-08-27T09:11:06Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Extracted `tangent_perturbation` pure helper in lib.rs using `scoring::perpendicular_basis`
- Made `perpendicular_basis` `pub(crate)` in scoring.rs for cross-module reuse
- Replaced ad-hoc `best_dir[1]*u2 - best_dir[2]*u1` block in `refine_once` with `tangent_perturbation` call
- Added `tangent_perturbation_is_perpendicular` test verifying |dot(dir, perp)| < 1e-5 for poles and tilted directions
- All 70 tests pass (1 ignored), including new perpendicularity test and existing refine_once invariants

## Task Commits

Work was already committed as part of consolidated Phase 7 commit:

1. **Task 1: RED — extract tangent_perturbation helper** — `e4ee0030` (feat — part of consolidated commit)
2. **Task 2: GREEN — make perpendicular_basis pub(crate) and fix helper** — `e4ee0030` (feat — part of consolidated commit)

**Plan metadata:** (pending — SUMMARY commit)

_Note: Both tasks were implemented in a single consolidated commit during Phase 7 planning. TDD gate commits (test/feat) are not present individually._

## Files Created/Modified
- `crates/orient/src/lib.rs` - Added `tangent_perturbation` helper, rewired `refine_once` to use it
- `crates/orient/src/scoring.rs` - Changed `perpendicular_basis` to `pub(crate)` visibility

## Decisions Made
- Reused existing `perpendicular_basis` from scoring.rs rather than creating a new perpendicular computation — same pattern already used by `misalignment_score` and `shadowed_overhang_fraction`
- Extracted as a pure free function (not a method) for testability and clarity

## Deviations from Plan

### Structural Deviation

**1. [Structural] File paths differ from plan**
- **Found during:** Execution
- **Issue:** Plan references `core/src/lib.rs` and `core/src/scoring.rs`, but Phase 8 workspace split moved code to `crates/orient/src/lib.rs` and `crates/orient/src/scoring.rs`
- **Fix:** All verification and execution used correct post-split paths
- **Files modified:** N/A (plan artifact, not code)
- **Verification:** All tests pass at correct paths
- **Committed in:** `e4ee0030` (already committed)

### Auto-fixed Issues

None — plan executed exactly as written (modulo path change).

---

**Total deviations:** 1 structural (path change due to Phase 8 workspace split)
**Impact on plan:** No impact — all code changes are correct and verified at post-split locations.

## Issues Encountered
None — all tests pass, perpendicularity constraint verified.

## User Setup Required
None — no external service configuration required.

## Next Phase Readiness
- Plan07-01 complete, ready for07-02 (H11 shadowed metric refinements)
- All existing tests green, no regressions introduced

## Self-Check: PASSED

- ✅ crates/orient/src/lib.rs exists
- ✅ crates/orient/src/scoring.rs exists
- ✅ Commit e4ee0030 exists (consolidated Phase 7 commit)
- ✅ All 70 tests pass (1 ignored)
- ✅ tangent_perturbation_is_perpendicular test passes

---
*Phase: 07-correctness-fixes-h11-scoring*
*Completed: 2026-08-27*
