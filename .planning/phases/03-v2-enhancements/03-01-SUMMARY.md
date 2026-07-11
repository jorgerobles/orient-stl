# Plan 03-01 SUMMARY: Rust WASM enhancements

**Phase:** 03 (v2-enhancements)
**Plan:** 01
**Status:** Complete

## What was built

### Task 1: Fibonacci sphere sampling + hull+sphere candidate generation

- **`generate_fibonacci_sphere(n: usize) -> Vec<[f32; 3]>`** in `core/src/candidates.rs` — generates `n` evenly-distributed unit direction samples on S² using the Fibonacci sphere algorithm (golden ratio φ, deterministic based on index). Called with n=200.

- **`generate_hull_plus_sphere(hull: &ConvexHull, n: usize, dedupe_angle_deg: f32) -> Vec<[f32; 3]>`** in `core/src/candidates.rs` — combines hull face normals with Fibonacci sphere samples, deduplicating against hull normals at the given angular threshold (default 3°). Preserves all hull normals and adds only Fibonacci directions > dedupe_angle_deg away from any hull normal. Expected direction count: more than hull-only, fewer than hull+200.

- **`prepare_data()` mode branching** in `core/src/lib.rs` — now accepts `mode: "hull_plus_sphere"` in addition to `"hull"`. When `hull_plus_sphere` is set, calls `generate_hull_plus_sphere(&hull, 200, config.dedupe_angle_deg)` instead of the two-step generate+deduplicate. The mode check rejects unknown modes as before.

### Task 2: Hill-climb `refine_orientation()` WASM function

- **`refine_orientation()`** in `core/src/lib.rs` — exported as `#[wasm_bindgen]` function:
  ```
  refine_orientation(
    positions: &[f32],    // flat per-triangle vertex positions (9 per triangle)
    normals: &[f32],      // per-triangle face normals (3 per triangle)
    areas: &[f32],        // per-triangle areas
    dir_x: f32, dir_y: f32, dir_z: f32,  // initial direction to refine
    critical_angle_deg: f32,  // overhang threshold
    iterations: u32,          // hill-climb iterations (clamped to 500 max)
  ) -> Vec<f32>  // [refined_dir_x, refined_dir_y, refined_dir_z, score]
  ```

  Implementation:
  - Reconstructs `MeshData` from flat arrays (no recomputation of normals/areas)
  - Initializes direction and scores it with `scoring::score_candidate()`
  - Hill-climb loop: generates random orthogonal perturbation using `js_sys::Math::random()` (no new crate deps needed)
  - Starts with 10° perturbation, decays by 0.95× each iteration (simulated annealing style)
  - After halfway, perturbation clamped to min 0.5° for convergence
  - Returns `vec![best_dir_x, best_dir_y, best_dir_z, best_score]` — always lower or equal penalty

## Verification

| Check | Result |
|-------|--------|
| `cargo check --target wasm32-unknown-unknown` | ✅ Passes (0 errors) |
| `cargo test` | ✅ 35/35 tests pass |
| `wasm-pack build core --target bundler --out-dir web/pkg` | ✅ Succeeds |
| Existing tests unaffected | ✅ All pass |

## Deviations from plan

- Hill-climb perturbation uses deterministic angle decay with floor (0.5°) instead of adaptive annealing — simpler, adequate for 50-iteration default
- `refine_orientation` re-normalizes input direction as safety measure
- Iterations capped at 500 per threat model (T-03-01)
