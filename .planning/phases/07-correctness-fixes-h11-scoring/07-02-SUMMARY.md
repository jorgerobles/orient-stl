---
phase: 07-correctness-fixes-h11-scoring
plan: 02
subsystem: stability
tags: [rust, center-of-mass, area-weighted, stability, convex-hull, signature-cleanup]

# Dependency graph
requires:
  - phase: 08-workspace-split
    provides: "crate-based workspace with crates/orient/ containing stability.rs"
provides:
  - "area-weighted center_of_mass helper replacing vertex-centroid bias"
  - "cleaned check_stability signature (hull param removed)"
affects: [stability, orientation-ranking]

# Tech tracking
tech-stack:
  added: []
  patterns: [area-weighted-centroid, signature-cleanup]

key-files:
  created: []
  modified:
    - crates/orient/src/stability.rs
    - crates/orient/src/main.rs
    - crates/orient/src/lib.rs

key-decisions:
  - "Area-weighted triangle-centroid average replaces vertex-centroid to eliminate triangulation-density bias"
  - "hull: &ConvexHull parameter removed from check_stability (internal API, safe signature change)"

patterns-established:
  - "Area-weighted centroid: Σ(area_i · centroid_i) / Σ(area_i) for surface-centroid approximation"

requirements-completed: []

# Metrics
duration: 1min
completed: 2026-08-27
---

# Phase 7 Plan 02: Center of Mass Correctness Fix Summary

**Area-weighted triangle-centroid average eliminates vertex-centroid triangulation-density bias in stability margin calculation**

## Performance

- **Duration:** <1 min (code pre-existing, verification only)
- **Started:** 2026-08-27T09:17:43Z
- **Completed:** 2026-08-27T09:17:43Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- Replaced vertex-centroid (Σ v / n) with area-weighted triangle-centroid average (Σ(area_i · centroid_i) / Σ(area_i)) in stability.rs
- Extracted `center_of_mass` helper function with area-weighting
- Rewired `check_stability` to use `center_of_mass(&mesh)` instead of inline vertex-sum loop
- Removed unused `hull: &ConvexHull` parameter from `check_stability` signature
- Updated all call sites: main.rs ×2, lib.rs ×1, stability.rs tests ×3

## Task Commits

Each task was committed atomically:

1. **Task 1: RED — center_of_mass helper with failing area-weighting test** - (pre-existing)
2. **Task 2: GREEN — area-weighted COM + rewire check_stability + drop hull param** - (pre-existing)

**Plan metadata:** `76d94e6` (docs: complete plan)

## Files Created/Modified
- `crates/orient/src/stability.rs` - Added area-weighted center_of_mass helper, cleaned check_stability signature
- `crates/orient/src/main.rs` - Updated check_stability call sites (lines 199, 282)
- `crates/orient/src/lib.rs` - Updated check_stability call site (line 470)

## Decisions Made
- Used area-weighted triangle-centroid average instead of vertex-centroid to eliminate triangulation-density bias
- Removed hull parameter from check_stability (internal API, safe signature change)

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Stability calculation now uses area-weighted centroid, reducing bias from uneven triangulation
- Ready for subsequent correctness and scoring plans in Phase 7

---
*Phase: 07-correctness-fixes-h11-scoring*
*Completed: 2026-08-27*
