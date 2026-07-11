# 01-02-SUMMARY: Rust Compute Core

**Status:** ✅ Complete
**Date:** 2026-07-11

## Deliverables

| Artifact | Status | Notes |
|---|---|---|
| `core/src/stl.rs` | ✅ | Binary STL parser via stl_io, returns `Vec<[f32;3]>` triangles |
| `core/src/mesh.rs` | ✅ | `MeshData` with per-triangle normals/areas/centroids, zero-area filtering |
| `core/src/hull.rs` | ✅ | Vendored incremental quickhull (f32), 326 lines |
| `core/src/candidates.rs` | ✅ | Hull-normal direction generation + angular deduplication |
| `core/src/scoring.rs` | ✅ | Area-weighted overhang penalty |
| `core/src/stability.rs` | ✅ | CoM projection vs contact footprint (267 lines) |
| `core/src/decimate.rs` | ✅ | Vertex sampling for hull input (added beyond plan) |
| `core/src/lib.rs` | ✅ | Orchestrator wiring modules together |

## Verification Results

| Check | Result |
|---|---|
| `cargo check --target wasm32-unknown-unknown` | ✅ passes |
| `wasm-pack build` | ✅ orient_core_bg.wasm (132KB) |
| WASM `prepare_data()` returns valid `OriData` | ✅ verified in browser |

## Deviations from Plan (IMPORTANT — architecture drift)

The plan specified WASM expose `compute_orientations()` running the **full** pipeline
(parse → precompute → hull → candidates → **score** → **stability**) and returning sorted
`Candidate[]`. The actual implementation **split the pipeline**:

- **WASM (`prepare_data`)** does: parse → precompute → hull → candidates → dedupe.
  Returns `OriData { positions, normals, areas, directions }` — raw geometry + directions.
- **JS (`web/src/compute.ts`, in Web Workers)** does: scoring, stability check,
  default yaw, height, decimation-for-score. See 01-03-SUMMARY and Phase 2.

**Rationale discovered during implementation:** keeping scoring/yaw/stability in JS
allowed rapid iteration on the scoring algorithm and multi-worker parallelism without
rebuilding WASM on each change. The WASM boundary is now the expensive one-time geometry
prep; per-candidate evaluation is parallelized across `navigator.hardwareConcurrency - 1`
workers in JS.

**Impact on requirements:** All Phase 1 success criteria still met — the pipeline runs
end-to-end and returns ranked candidates — but the boundary lives in JS, not Rust. A
future refactor (Phase 3 or later) may move scoring back into WASM if JS proves too slow.

## Commands

```bash
cargo check --target wasm32-unknown-unknown
wasm-pack build core --target bundler --out-dir web/pkg
```
