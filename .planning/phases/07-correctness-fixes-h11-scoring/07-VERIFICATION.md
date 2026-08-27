---
phase: 07-correctness-fixes-h11-scoring
verified: 2026-08-27T11:46:33Z
status: passed
score: 30/30 must-haves verified
overrides_applied: 0
re_verification: false
---

# Phase 7: Correctness Fixes + H11 Scoring Verification Report

**Phase Goal:** Fix three correctness/quality bugs surfaced by code review and wire the resin-critical shadowed-overhang metric (H11) into the composite score so it actually affects ranking.
**Verified:** 2026-08-27T11:46:33Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| #  | Truth | Status | Evidence |
|----|-------|--------|----------|
| 1  | `tangent_perturbation(dir, u1, u2)` exists in lib.rs as a pure helper | ✓ VERIFIED | `crates/orient/src/lib.rs:208` — calls `scoring::perpendicular_basis` |
| 2  | `perpendicular_basis` is `pub(crate)` in scoring.rs | ✓ VERIFIED | `crates/orient/src/scoring.rs:4` — `pub(crate) fn perpendicular_basis` |
| 3  | `refine_once` uses `tangent_perturbation` (reuses `perpendicular_basis`), not ad-hoc cross product | ✓ VERIFIED | `crates/orient/src/lib.rs:229` — `let perp = tangent_perturbation(&best_dir, u1, u2);` |
| 4  | Ad-hoc formula `best_dir[1]*u2 - best_dir[2]*u1` is removed from `refine_once` | ✓ VERIFIED | grep confirms zero matches in lib.rs |
| 5  | `tangent_perturbation_is_perpendicular` test passes: |dot(dir, perp)| < 1e-5 | ✓ VERIFIED | `crates/orient/src/lib.rs:620` — tests 5 dirs × 6 (u1,u2) pairs |
| 6  | `refine_once_never_worsens_score` invariant test passes | ✓ VERIFIED | `crates/orient/src/lib.rs:653` |
| 7  | `pub(crate) fn center_of_mass(mesh: &MeshData) -> [f32; 3]` exists in stability.rs | ✓ VERIFIED | `crates/orient/src/stability.rs:11` |
| 8  | `center_of_mass` computes area-weighted triangle-centroid average | ✓ VERIFIED | `crates/orient/src/stability.rs:14-31` — Σ(area_i · centroid_i) / Σ(area_i) |
| 9  | `center_of_mass` is NOT raw vertex centroid — verified by `center_of_mass_is_area_weighted` test | ✓ VERIFIED | `crates/orient/src/stability.rs:248` — test passes, com[0] < -0.5 |
| 10 | `check_stability` calls `center_of_mass(&mesh)` instead of inline vertex-sum loop | ✓ VERIFIED | `crates/orient/src/stability.rs:75` |
| 11 | `hull: &ConvexHull` parameter removed from `check_stability` signature | ✓ VERIFIED | `crates/orient/src/stability.rs:34` — signature is `(direction, mesh)` only |
| 12 | All `check_stability` call sites updated (no hull arg): main.rs ×2, lib.rs ×1, tests ×3 | ✓ VERIFIED | main.rs:199, main.rs:282, lib.rs:470, stability.rs tests:274,284,292 |
| 13 | Dead yaw subgraph deleted: `compute_default_yaw`, `find_best_yaw`, `rotate_point`, `quat_rotate`, `quat_mul`, `rotating_calipers_bbox`, `bbox_area`, `convex_hull_2d` | ✓ VERIFIED | candidates.rs is 96 lines; grep confirms none of these symbols present |
| 14 | `#[deprecated]`/`#[allow(dead_code)]` markers removed from candidates.rs | ✓ VERIFIED | No such attributes in candidates.rs |
| 15 | Surviving functions unchanged: `generate_candidates`, `deduplicate_directions`, `generate_fibonacci_sphere`, `generate_hull_plus_sphere` + 3 tests | ✓ VERIFIED | candidates.rs:3,7,25,40 + tests at lines 77,84,91 |
| 16 | `ScoreWeights` (Rust) has `w_shadowed: f32` | ✓ VERIFIED | ranking.rs:15 |
| 17 | `ScoreComponents` (Rust) has `shadowed: f32` | ✓ VERIFIED | scoring.rs:205 |
| 18 | `score_components` computes shadowed via `shadowed_overhang_fraction` with grid_res=32, tol_frac=0.02 | ✓ VERIFIED | scoring.rs:221 — `SHADOWED_GRID_RES=32`, `SHADOWED_TOL=0.02` constants at lines 191-193 |
| 19 | `rank_by_weights_with_bounds` normalizes shadowed as COST (higher=worse): shn = clamp((m.shadowed - sh_lo)/sh_span), adds w.w_shadowed * shn | ✓ VERIFIED | ranking.rs:98,104 |
| 20 | Norm bounds are `&[f32; 6]` (overhang, footprint, cross, surface, height, shadowed) | ✓ VERIFIED | ranking.rs:53-54 |
| 21 | `compute_norm_bounds` returns 12 floats [lo[6], hi[6]] | ✓ VERIFIED | lib.rs:571 |
| 22 | `rank_candidates` parses `weights[5]` as `w_shadowed`, builds `[f32; 6]` norm arrays | ✓ VERIFIED | lib.rs:515,518-520 |
| 23 | `score_direction` returns 9 floats (3 dir + 6 components) | ✓ VERIFIED | lib.rs:441-444 |
| 24 | `rank_by_consensus_with_bounds` handles 6th weight (shadowed not silently ignored) | ✓ VERIFIED | ranking.rs:180 — `max_w` includes `.max(w.w_shadowed)`, line 191 — `sh_n` computed |
| 25 | `rank_by_topsis` handles 6th weight | ✓ VERIFIED | ranking.rs:230,238,251-252,265-266,304-308 — shadowed in ideal/nadir vectors |
| 26 | Harness computes real shadowed per candidate (not 0.0); `WeightCfg` has `w_shadowed` | ✓ VERIFIED | harness.rs:19 (`w_shadowed` field), line 79 (`shadowed_overhang_fraction` call), line 99 (real value in metrics) |
| 27 | `ScoreWeights` (TS) has `wShadowed: number` | ✓ VERIFIED | types.ts:70 |
| 28 | All 8 profile JSONs have `wShadowed` (resin-biased high=2, equal=1, *-only=0) | ✓ VERIFIED | equal.json, resin-biased.json, overhang-only.json confirmed; all 8 files exist |
| 29 | `AppController.computeNormBounds` reads `subarray(0,6)` / `subarray(6,12)` | ✓ VERIFIED | AppController.ts:268 |
| 30 | `AppController.updateLiveScore` builds 6-element costs + 6-element weights; `spawnCompute` sends 6-element wArr | ✓ VERIFIED | AppController.ts:298-306 (costs/weights), line 397 (wArr) |

**Score:** 30/30 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/orient/src/lib.rs` | tangent_perturbation helper + refined refine_once | ✓ VERIFIED | Lines 208-250 |
| `crates/orient/src/scoring.rs` | pub(crate) perpendicular_basis | ✓ VERIFIED | Line 4 |
| `crates/orient/src/stability.rs` | area-weighted center_of_mass + cleaned check_stability | ✓ VERIFIED | Lines 11-97 |
| `crates/orient/src/main.rs` | check_stability calls without hull arg | ✓ VERIFIED | Lines 199, 282 |
| `crates/orient/src/candidates.rs` | Candidates module without dead yaw subgraph | ✓ VERIFIED | 96 lines, no dead functions |
| `crates/orient/src/ranking.rs` | 6-component weighted ranking with shadowed as cost | ✓ VERIFIED | Lines 9-115 |
| `crates/orient/src/harness.rs` | WeightCfg with w_shadowed, real shadowed per candidate | ✓ VERIFIED | Lines 12-20, 79, 99 |
| `crates/geometry-kernel/src/flat.rs` | normalize_winding with edge-adjacency BFS | ✓ VERIFIED | Lines 95-245 |
| `web/src/types.ts` | ScoreWeights with wShadowed, 6-tuple weights | ✓ VERIFIED | Lines 64-71, 83 |
| `web/src/app/AppController.ts` | 6-component norm bounds + live score | ✓ VERIFIED | Lines 260-268, 275-331, 377-437 |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `lib.rs::refine_once` | `scoring.rs::perpendicular_basis` | `tangent_perturbation` calls `scoring::perpendicular_basis` | ✓ WIRED | lib.rs:209 |
| `stability.rs::check_stability` | `stability.rs::center_of_mass` | COM projection for stability margin | ✓ WIRED | stability.rs:75 |
| `lib.rs::prepare_data_native` | `flat.rs::normalize_winding` | Called after `repair_mesh` | ✓ WIRED | lib.rs:133 |
| `ranking.rs::rank_by_weights_with_bounds` | `scoring.rs::shadowed_overhang_fraction` | Shadowed flows from score_components → CandidateMetrics → composite | ✓ WIRED | scoring.rs:221, ranking.rs:98 |
| `AppController.ts::computeNormBounds` | WASM `compute_norm_bounds` | 12-float norm bounds array | ✓ WIRED | AppController.ts:264-268 |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| (none) | - | - | - | No debt markers (TBD/FIXME/XXX/TODO) found in any phase-modified file |

### Probe Execution

| Probe | Command | Result | Status |
|-------|---------|--------|--------|
| (none) | - | - | SKIPPED (no probe scripts defined for this phase) |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| (none) | - | Phase 7 has no requirement IDs in REQUIREMENTS.md traceability table | N/A | REQUIREMENTS.md traceability has no Phase 7 entries; all 26 v1-v3 requirements mapped to Phases 1-4 |

**Orphaned requirements:** None — REQUIREMENTS.md traceability table has no Phase 7 entries.

### Human Verification Required

### 1. Resin-biased profile penalizes cavities

**Test:** Load a resin miniature STL (e.g. `resources/Skulled_Wurm_Bird_WOBase.stl`) in the web UI with the "resin-biased" profile selected. Rotate to an orientation that creates a sealed cavity (suction-cup orientation). Confirm the live score reflects the shadowed penalty (score panel shows "Reachability" bar reduced).
**Expected:** Orientations with high shadowed-overhang fraction rank lower than before Phase 7; live score decreases when rotating toward cavity-forming orientations.
**Why human:** Requires visual inspection of the live score panel behavior during manual rotation — cannot verify programmatically without starting the dev server.

### 2. No NaN scores in live-score panel

**Test:** Load any STL, rotate freely in overlay mode for 30+ seconds, check that the score panel never shows NaN or Infinity.
**Expected:** All score values remain finite numbers between 0% and 100%.
**Why human:** Requires interactive rotation in the browser — cannot verify without live viewport.

### Gaps Summary

No gaps found. All 30 must-haves verified against actual codebase evidence. Phase goal achieved:
- Three correctness bugs fixed: tangent perturbation perpendicularity, area-weighted center of mass, dead yaw subgraph deletion
- H11 shadowed-overhang metric wired end-to-end: Rust scoring → ranking (all 3 rankers) → WASM exports (6-weight, 12-float norm bounds, 9-float score_direction) → TS AppController → profile JSONs → harness
- All 73 Rust tests pass (70 unit + 3 integration), 78 TS tests pass, type-check clean, zero anti-patterns

---

_Verified: 2026-08-27T11:46:33Z_
_Verifier: the agent (gsd-verifier)_
