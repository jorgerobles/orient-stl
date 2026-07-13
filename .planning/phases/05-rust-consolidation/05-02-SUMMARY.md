---
phase: 05
plan: 02
subsystem: core
tags: [wasm-exports, cli, native-pipeline, ranking, selection]
requires: [05-01]
provides: [wasm-score-all, wasm-rank, wasm-norm-bounds, wasm-select-diverse, prepare-data-native, cli-binary]
affects: [web/pkg, web/src/compute.ts]
tech-stack:
  added:
    - clap 4 (CLI arg parsing, derive-based)
    - serde_json 1 (JSON output for CLI)
  patterns:
    - Dual-target: WASM exports gated by `wasm` feature, CLI binary gated by `cli` feature
    - Shared pipeline: all STL→mesh→hull→candidates logic in `prepare_data_native` (ungated)
key-files:
  created:
    - core/src/main.rs (CLI runner, 247 lines)
  modified:
    - core/src/lib.rs (feature gates, new exports, pub items for CLI)
    - core/src/harness.rs (uses ranking module)
    - core/Cargo.toml ([[bin]] target)
    - core/src/mesh.rs (pub MeshData)
    - core/src/hull.rs (pub ConvexHull)
    - core/src/scoring.rs (pub scoring functions)
    - core/src/stability.rs (pub stability functions)
    - core/src/rng.rs (pub Rng + seed_from_direction)
    - core/src/ranking.rs (pub ranking functions + types)
    - core/src/selection.rs (pub merge_candidates)
    - core/src/yaw.rs (pub full_quaternion)
    - core/src/decimate.rs (pub sample_for_hull)
decisions:
  - "Make internal modules pub for binary crate access — orient-stl is an internal tool, not a published library"
  - "prepare_data_native returns OriData (not MeshData+Hull) — CLI reconstructs hull from returned positions"
metrics:
  tasks: 3/3
  duration: ~25 min
  completed: 2026-07-13
---

# Phase 5 Plan 2: WASM Exports + Native CLI Runner — Summary

**One-liner:** Added 4 new WASM-bindgen exports (score_all_directions, rank_candidates, compute_norm_bounds, select_diverse) matching the JS compute.ts architecture, an ungated `prepare_data_native` shared pipeline, and a native CLI runner under the `cli` feature.

## Tasks Completed

### Task 1: Add score_all_directions + rank_candidates + compute_norm_bounds + select_diverse (type: auto)

- Feature-gated existing `refine_orientation`, `refine_orientation_batch`, `score_orientation` behind `#[cfg(feature = "wasm")]`
- Added `score_all_directions`: per-direction loop returning N×13 floats (quaternion + overhang + footprint + max_cross + surface + height + shadowed + stability triples)
- Added `rank_candidates`: dispatches to `weights` / `consensus` / `topsis` ranking from `ranking` module
- Added `compute_norm_bounds`: samples ~30 directions for min/max normalization bounds
- Added `select_diverse`: wraps `selection::merge_candidates` via WASM FFI
- Dual-target verification: `cargo test --lib` (all 80 pass) + `cargo build --no-default-features` (no errors)
- **Deviation (Rule 1):** Fixed `Option<&js_sys::Function>` → `Option<js_sys::Function>` (wasm-bindgen trait bound) and prefixed unused `exclude_unstable` with underscore
- **Commit:** `643bd39`

### Task 2: Add prepare_data_native, drop self-referential tests, update harness (type: auto)

- Added `prepare_data_native(bytes, mode, dedupe_angle)` → `Result<OriData, String>` — ungated, called by both WASM `prepare_data` and the CLI
- Refactored existing `prepare_data` to call `prepare_data_native` internally (shared parsing + precompute + hull + candidate generation)
- Dropped `score_orientation_zero_iterations_matches_raw_score` and `cube_metrics_match_score_components` (self-referential tests that duplicate the internal implementation)
- Updated `harness.rs` to use `ranking::rank_by_weights` instead of inline weighted-merge ranking
- Gated 9 WASM-dependent tests behind `#[cfg(feature = "wasm")]` so `cargo test --no-default-features --lib` passes (69 tests vs 78 with wasm)
- **Commit:** `b978c64`

### Task 3: Native CLI runner (type: auto)

- Created `core/src/main.rs` with clap-derive argument parsing:
  - `--stl` (input STL path), `--mode` (hull/hull_plus_sphere), `--critical-angle`, `--dedupe-angle`, `--refine-iters`
  - `--method` (weights/consensus/topsis), `--weights` (5 comma-separated), `--exclude-unstable`
  - `--max-candidates`, `--min-angle`, `--output` (optional JSON file)
- Pipeline: `prepare_data_native` → per-direction scoring (all 13 metrics) → ranking → `merge_candidates` → JSON output
- Made internal types/functions `pub` for binary crate access, added `[[bin]]` target to `Cargo.toml` (gated by `cli` feature)
- Verified: `cargo build --features cli` succeeds, `orient --stl test-tetrahedron.stl --max-candidates 3` produces valid JSON
- **Deviation (Rule 3):** Needed to make all module-level items `pub` — the binary crate (`main.rs`) is a separate compilation unit from the lib crate, and `pub(crate)` items are invisible to it
- **Commit:** `b08ebf8`

## Deviations from Plan

### Rule 1 — Bug fixes

1. **Fixed `Option<&js_sys::Function>` trait bound** (`core/src/lib.rs`)
   - Found during: Task 1
   - Issue: `wasm_bindgen` macro doesn't implement `OptionFromWasmAbi` for `&js_sys::Function`
   - Fix: Changed `Option<&js_sys::Function>` to `Option<js_sys::Function>`, and `if let Some(cb) = progress` to `if let Some(ref cb) = progress`

2. **Unused parameter `exclude_unstable`** (`core/src/lib.rs`)
   - Found during: Task 1 build
   - Fix: Prefixed with underscore

### Rule 3 — Blocking issues

1. **Binary crate visibility** (`core/src/lib.rs` + 7 module files)
   - Found during: Task 3
   - Issue: `main.rs` is a separate crate from `lib.rs` — `pub(crate)` functions/types are invisible to the binary
   - Fix: Made internal modules `pub` (7 modules), and added `pub` visibility to ~20 functions/types including `MeshData`, `ConvexHull`, all scoring/ranking/stability/yaw/RNG functions

### Scope boundary notes
- Pre-existing warnings (`w2v` unused in scoring.rs, `lo` unused in ranking.rs, `angle_between` dead in selection.rs) are tracked but not fixed — out of scope for this plan

## Self-Check: PASSED

- ✅ `core/src/main.rs` exists (247 lines)
- ✅ `core/Cargo.toml` has `[[bin]]` section with required-features = ["cli"]
- ✅ Commit `643bd39` exists (Task 1 — WASM exports)
- ✅ Commit `b978c64` exists (Task 2 — prepare_data_native, harness, tests)
- ✅ Commit `b08ebf8` exists (Task 3 — CLI runner)
- ✅ `cargo test --lib` passes (78 tests)
- ✅ `cargo test --no-default-features --lib` passes (69 tests)
- ✅ `cargo build --features cli` succeeds
- ✅ `orient --stl test-tetrahedron.stl` produces valid JSON

## Threat Flags

None — all new surface (WASM exports, CLI binary) gates behind features and follows existing patterns.

## Success Criteria

| Criterion | Status |
|-----------|--------|
| score_all_directions is gated behind wasm feature | ✅ |
| rank_candidates calls ranking module internally | ✅ |
| compute_norm_bounds samples directions correctly | ✅ |
| select_diverse wraps merge_candidates | ✅ |
| prepare_data_native is ungated (no `#[cfg]` or `#[wasm_bindgen]`) | ✅ |
| prepare_data calls prepare_data_native | ✅ |
| Self-referential tests removed | ✅ (2 dropped) |
| Harness uses ranking::rank_by_weights | ✅ |
| cargo test --no-default-features --lib passes | ✅ (69 tests) |
| CLI binary builds with `cargo build --features cli` | ✅ |
| CLI produces valid JSON output | ✅ |
| JSON includes meta, candidates (ranked), selected (diverse) | ✅ |
