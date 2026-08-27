---
phase: 07-correctness-fixes-h11-scoring
plan: 05
subsystem: geometry-kernel
tags: [winding, normal, repair, mesh, centroid, bfs]

# Dependency graph
requires:
  - phase: 08-workspace-split
    provides: "geometry-kernel crate with flat-array repair functions"
provides:
  - "normalize_winding function in geometry-kernel/flat.rs"
  - "Edge-adjacency BFS + centroid voting winding normalization"
  - "WASM export normalize_winding_tris"
  - "Integration in prepare_data_native pipeline"
affects: [07-correctness-fixes-h11-scoring]

# Tech tracking
tech-stack:
  added: []
  patterns: [edge-adjacency-bfs, centroid-voting, flat-array-repair]

key-files:
  created: []
  modified:
    - crates/geometry-kernel/src/flat.rs — normalize_winding implementation + tests
    - crates/orient/src/lib.rs — wiring in prepare_data_native + WASM export

key-decisions:
  - "Edge-adjacency BFS + centroid voting over simple per-triangle centroid heuristic"
  - "MIN_COMPONENT_VOTE threshold (4) prevents degenerate component mis-orientation"

patterns-established:
  - "Flat-array repair functions: accept &mut Vec<f32>, return u32 flip count"
  - "Centroid voting: compute component centroid, vote outward per triangle, flip majority-inward"

requirements-completed: []

# Metrics
duration: 1min
completed: 2026-08-27
---

# Phase 7 Plan 05: Winding Normalization Summary

**Edge-adjacency BFS + centroid voting normalizes triangle winding in flat-array repair pipeline**

## Performance

- **Duration:** <1 min (pre-existing implementation verified)
- **Started:** 2026-08-27T09:39:06Z
- **Completed:** 2026-08-27T09:40:00Z
- **Tasks:** 4 (all pre-existing, verified)
- **Files modified:** 2

## Accomplishments
- Verified `normalize_winding` exists in `crates/geometry-kernel/src/flat.rs` with edge-adjacency BFS propagation + per-component centroid voting
- Verified wiring in `prepare_data_native` at `crates/orient/src/lib.rs:133` (called after `repair_mesh`)
- Verified WASM export `normalize_winding_tris` at `crates/orient/src/lib.rs:288`
- All 105 workspace tests pass including 7 normalize_winding-specific tests

## Task Commits

All code was implemented in prior waves. No new commits produced — implementation already satisfied plan requirements.

1. **Task 1 (RED):** Tests pre-exist — `normalize_winding_empty`, `normalize_winding_single_triangle`, `normalize_winding_two_triangles_inverted`, `normalize_winding_thin_shell`, `normalize_winding_degenerate_edge_skipped`, `normalize_winding_chain_propagation`, `normalize_winding_disconnected_components`
2. **Task 2 (GREEN):** Implementation pre-exists — edge-adjacency BFS + centroid voting in `flat.rs:95-245`
3. **Task 3 (REFACTOR):** Wiring pre-exists — `geometry_kernel::flat::normalize_winding(&mut flat)` in `lib.rs:133`
4. **Task 4 (Verify):** All 105 tests pass; broken.stl not available for CLI regression test (test-tetrahedron.stl available)

## Files Created/Modified
- `crates/geometry-kernel/src/flat.rs` — normalize_winding implementation (lines 95-245) + 7 test cases (lines 592-671)
- `crates/orient/src/lib.rs` — pipeline wiring (line 133) + WASM export (lines 288-292)

## Decisions Made
- Edge-adjacency BFS + centroid voting chosen over simple per-triangle centroid heuristic for robustness on disconnected components and thin shells
- MIN_COMPONENT_VOTE threshold of 4 prevents degenerate single/double-triangle components from being mis-oriented

## Deviations from Plan

### Implementation Already Existed

**1. Plan described simple centroid heuristic; codebase has sophisticated edge-adjacency + centroid voting**
- **Found during:** Task 1 (RED — tests already existed)
- **Issue:** normalize_winding was implemented in an earlier wave with a more robust algorithm than the plan specified
- **Fix:** No action needed — existing implementation satisfies all plan must_haves
- **Files modified:** None (pre-existing)
- **Verification:** All 7 normalize_winding tests pass, all 105 workspace tests pass
- **Commit:** Part of earlier geometry-kernel work (52b003b7, fdc7e649)

**2. Plan referenced `core/src/repair.rs` and `core/src/lib.rs`; files are now at `crates/geometry-kernel/src/flat.rs` and `crates/orient/src/lib.rs`**
- **Found during:** Task 3 (REFACTOR — wiring verification)
- **Issue:** Plan written before workspace split (Phase 8); file paths are outdated
- **Fix:** No action needed — files exist at correct post-split locations
- **Verification:** `grep` confirms function exists and is wired correctly
- **Commit:** N/A (path drift, not code issue)

---

**Total deviations:** 2 (both pre-existing state, no action required)
**Impact on plan:** None — all requirements satisfied by existing code.

## Issues Encountered
None — implementation pre-existed and all tests pass.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Winding normalization complete and verified in the repair pipeline
- Ready for Phase 8 completion or next phase work

## Self-Check: PASSED

- [x] SUMMARY.md exists
- [x] `normalize_winding` exists in `crates/geometry-kernel/src/flat.rs:95` (returns u32)
- [x] Wired in `prepare_data_native` at `crates/orient/src/lib.rs:133`
- [x] WASM export at `crates/orient/src/lib.rs:288`
- [x] All 105 workspace tests pass (0 failures)
- [x] 7 normalize_winding-specific tests pass

---
*Phase: 07-correctness-fixes-h11-scoring*
*Completed: 2026-08-27*
