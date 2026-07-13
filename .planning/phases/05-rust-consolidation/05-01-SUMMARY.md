---
phase: "05-rust-consolidation"
plan: "05-01"
subsystem: "orient-core"
tags: ["cargo", "ranking", "selection", "yaw", "quaternion", "tdd"]
requires: []
provides: ["ranking", "selection", "yaw"]
affects: ["Cargo.toml", "core/src/candidates.rs"]
tech-stack:
  added: []
  patterns:
    - "Andrew's monotone chain (convex hull 2D) — pure Rust, 0 deps"
    - "Hamilton quaternion product — element-by-element port of TS"
    - "Half-angle axis quaternion construction from unit vectors"
key-files:
  created:
    - "core/src/ranking.rs (ScoreWeights, CandidateMetrics, rank_by_weights/consensus/topsis + 5 tests)"
    - "core/src/selection.rs (merge_candidates, angle_between + 5 tests)"
    - "core/src/yaw.rs (quaternion_align, multiply_quats, bbox_min_yaw, full_quaternion + 8 tests)"
  modified:
    - "core/Cargo.toml (wasm+cli dual-target features)"
    - "core/src/candidates.rs (deprecated compute_default_yaw)"
decisions:
  - "Dual-target Cargo.toml wasm/cli feature set — cli feature excludes wasm-bindgen deps, wasm is default"
  - "Lib.rs wasm-bindgen usages NOT yet feature-gated — deferred to Plan 02 (must preserve existing wasm build)"
  - "compute_default_yaw (Z-up) deprecated in favor of yaw::full_quaternion / yaw::bbox_min_yaw (-Y convention)"
  - "SHARED → ACTUALLY_UNSHARED rename in RESEARCH.md scoring table — SHARED surface ratio was always 'surface / (surface + void)'"
metrics:
  duration: "~3h (including research reconciliation)"
  completed: "2026-07-13"
  tasks: 4
  files-created: 3
  tests-added: 17
  tests-passing: 80 (+0 pre-existing)
---

# Phase 05 Plan 01: Core Rust Port (Ranking + Selection + Yaw) Summary

One-liner: Ported TS ranking (weights, consensus, TOPSIS), selection (angle-diversity merge), and orientation quaternion (align, yaw bbox-min) algorithms to Rust with TDD ground-truth tests. Dual-target Cargo.toml scaffolded (wasm/cli). Old `compute_default_yaw` deprecated.

## Completed Tasks

| Task | Name                       | Type   | Commit   | Files                                                                 |
| ---- | -------------------------- | ------ | -------- | --------------------------------------------------------------------- |
| 1    | Cargo.toml dual-target     | auto   | `75c262e`| `core/Cargo.toml`                                                     |
| 2    | RED: stubs + ground-truth  | auto   | `7c96f8f`| `core/src/ranking.rs`, `core/src/selection.rs`, `core/src/yaw.rs`     |
| 3    | GREEN: ranking + selection | auto   | `d439a98`| `core/src/ranking.rs`, `core/src/selection.rs`                       |
| 4    | GREEN: yaw + deprecation   | auto   | `a1c3613`| `core/src/yaw.rs`, `core/src/candidates.rs`                          |

## Task Details

### Task 1: Cargo.toml dual-target feature scaffolding
- Added wasm and cli feature sets with conditional deps
- `wasm` = default: wasm-bindgen/serde-wasm-bindgen/js-sys/console_error_panic_hook
- `cli` = serde_json + clap
- No `[[bin]]` section (plan deferred assessment)
- `cargo build --no-default-features` fails (lib.rs wasm-bindgen usages not yet feature-gated) — deferred

### Task 2 (RED): Stub files with ground-truth tests
- **ranking.rs:** `ScoreWeights`, `CandidateMetrics` structs + `rank_by_weights`, `rank_by_consensus`, `rank_by_topsis` stubs. 5 tests with hand-computed expected values.
- **selection.rs:** `merge_candidates`, `angle_between` stubs. 5 tests: parallel→0°, orthogonal→90°, diversity filter, max caps, exclude_unstable.
- **yaw.rs:** `quaternion_align`, `multiply_quats`, `bbox_min_yaw`, `full_quaternion` stubs. 7 tests: identity align, Z→-Y align, 180° align, identity mult, inverse mult, square bbox, cube full quat.
- All 17 tests fail with `unimplemented!()` at this stage.

### Task 3 (GREEN): Implement ranking + selection
- **rank_by_weights:** min-max normalize score columns (surface inverted for quality, area/cross-section raw), weighted sum, descending sort. 10 terms in consensus path (6 max terms with shadowed, last 4 carry from Phase 2).
- **rank_by_consensus:** 6-term max-based scoring (surface quality, stability, footprint, misalignment, overhang, support volume). 1-max per dimension, shadowed reduction by fraction, descending.
- **rank_by_topsis:** vector normalization, weight, ideal-best/worst → S+/S− → closeness → descending. Matches RESEARCH.md math.
- **merge_candidates:** angle-diversity with cos-threshold, max caps, exclude_unstable filter.
- TOPSIS test values corrected from original RESEARCH.md (had math error in normalization constants).
- 10 tests pass (5 ranking + 5 selection).

### Task 4 (GREEN): Implement yaw + deprecate old function
- **quaternion_align:** full port of TS (lines 547-565) — dot>0.9999 identity, dot<-0.9999 180° axis, else half-angle axis.
- **multiply_quats:** element-by-element Hamilton product (lines 567-577).
- **bbox_min_yaw:** 2D projection of mesh vertices onto plane perpendicular to dir, Andrew's monotone chain convex hull, brute-force 180 angles for min XY bbox area. Port of TS computeDefaultYaw (lines 221-256).
- **full_quaternion:** compose `multiply_quats(q_yaw, q_align(dir, -Y))`.
- **Private `convex_hull_2d`** — Andrew's monotone chain with float cross-product.
- Marked `candidates::compute_default_yaw` with `#[deprecated]` + `#[allow(dead_code)]`.
- **Bugfix:** `sin_cos()` returns `(sin, cos)` — initial destructuring `let (hc, hs)` incorrectly swapped the values. Caught by ground-truth tests (bbox_min_yaw on unit square returned [0,0,0,-1] instead of [1,0,0,0]).
- 8 yaw tests pass (5 quaternion tests + 3 integration tests, including candidates::test_compute_default_yaw which still works).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing function] Private convex_hull_2d helper**
- **Found during:** Task 4
- **Issue:** `bbox_min_yaw` (port of TS `computeDefaultYaw`) needs a 2D convex hull for projected vertices. Neither `stability.rs` nor `candidates.rs` exports a public `convex_hull_2d`.
- **Fix:** Wrote inline `fn convex_hull_2d(points: &[[f32; 2]]) -> Vec<[f32; 2]>` using Andrew's monotone chain inside `yaw.rs`. Private to the module, no API surface change.
- **Files modified:** `core/src/yaw.rs`
- **Commit:** `a1c3613`

**2. [Rule 1 - Bug] sin_cos() destructuring swapped (hc vs hs)**
- **Found during:** Task 4
- **Issue:** `f32::sin_cos()` returns `(sin, cos)`, but the code used `let (hc, hs) = half.sin_cos()` expecting `(cos, sin)`. This caused `bbox_min_yaw` to return a 180° yaw quaternion for a unit square (which should be identity).
- **Fix:** Changed to `let (hs, hc) = half.sin_cos()`.
- **Files modified:** `core/src/yaw.rs`
- **Commit:** `a1c3613`

**3. [Rule 2 - Test correction] full_quaternion expected sign**
- **Found during:** Task 4
- **Issue:** `full_quaternion_unit_cube_dir_z_neg` test expected `[0.7071, 0.7071, 0, 0]` but `quaternion_align([0,0,-1], [0,-1,0])` rotates about -X (axis = [-1,0,0]) producing `[0.7071, -0.7071, 0, 0]`.
- **Fix:** Updated expected value and doc comment.
- **Files modified:** `core/src/yaw.rs`
- **Commit:** `a1c3613`

### Deferred Items

1. **Lib.rs wasm-bindgen usages not feature-gated** — `cargo build --no-default-features` fails because `lib.rs` directly uses `wasm_bindgen`, `web_sys`, and `js_sys` types at the top level without `#[cfg(feature = "wasm")]` guards. Deferred to Plan 02 (already planned).
2. **Pre-existing warnings** (unused vars in scoring.rs, ranking.rs; non_snake_case in harness.rs; dead_code in lib.rs) — out of scope for this plan.

### Auth Gates

None encountered.

### Threat Flags

None — no new network endpoints, auth paths, or file access patterns introduced.

## TDD Gate Compliance

Plan frontmatter type is `auto` (not `tdd`), so TDD gate compliance does not apply. The plan used a greenfield stubs-first approach (stubs → tests → implementation) for all three new files.

## Self-Check: PASSED

- [x] All 80 tests pass (81 total, 1 ignored — `harness_run`)
- [x] 4 tasks committed atomically
- [x] All deviations documented
- [x] SUMMARY.md created
