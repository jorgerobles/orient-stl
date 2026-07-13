# Phase 5: Consolidate All Calculations in Rust - Context

**Gathered:** 2026-07-13
**Status:** Ready for planning
**Source:** Direct user discussion (no discuss-phase needed — decisions are fully locked)

<domain>
## Phase Boundary

This phase consolidates ALL calculation logic into Rust, eliminating the current TS/Rust duplication. Every metric, ranking algorithm, selection logic, and yaw computation moves to Rust. A CLI binary enables independent verification outside the browser. TS becomes a thin rendering layer that calls WASM functions and displays results.

**In scope:**
- Move all TS metric implementations to Rust (overhang, footprint, cross-section, surface quality, height, shadowed overhang)
- Move all TS ranking algorithms to Rust (weighted sum, consensus, TOPSIS)
- Move candidate selection (angle-diversity merge) to Rust
- Move yaw optimization (bbox-minimizing) to Rust
- Create a single WASM `score_all_directions` function that replaces the worker-based `computeSlice` pipeline
- Create a Rust CLI binary for independent verification
- Replace all self-referential tests with ground-truth (hand-computed) tests
- Delete all TS metric/ranking tests
- Strip TS compute.ts to type definitions + WASM call wrappers only

**Out of scope:**
- UI/UX changes (Phase 4)
- New metrics or ranking algorithms
- Thumbnail strip, favorites, ZIP export (Phase 4)
- WASM multithreading (SharedArrayBuffer/rayon) — single-threaded WASM is sufficient
</domain>

<decisions>
## Implementation Decisions

### Architecture: Single WASM Call (LOCKED)
- Replace the worker-based `computeSlice` pipeline with a single WASM `score_all_directions` function that scores ALL directions in Rust
- Workers are eliminated entirely — one WASM call handles the full candidate search
- Progress callbacks via JS (WASM calls a JS callback function periodically)
- Rationale: Simpler, one source of truth, no worker coordination complexity. WASM is fast enough for single-threaded scoring of ~50-300 directions.

### CLI Structure: Binary in Same Crate (LOCKED)
- Add `src/bin/cli.rs` to the `orient-core` crate
- Shares all code with the WASM build via the `rlib` crate type (already configured)
- Usage: `cargo run --bin cli -- input.stl --angle 30 --profile resin-biased --ranker topsis`
- Outputs JSON with all candidates, metrics, and composite scores
- Can be used for regression testing and correctness verification

### Test Strategy: Delete All TS Metric/Ranking Tests (LOCKED)
- Remove `compute.test.ts` entirely (52 tests for TS metric/ranking duplicates)
- Keep TS test files: `quaternion.test.ts`, `rotate.test.ts`, `convention.test.ts`, `centering.test.ts` (display/rendering math)
- All metric, ranking, and selection tests live in Rust (`cargo test`)
- Ground-truth tests only — hand-computed expected values from known geometry

### Ground-Truth Test Requirements (LOCKED)
- Every test must use a mesh with known geometry where the expected value can be computed by hand (arithmetic, not by running the implementation)
- Tests that compare an implementation to itself (consistency tests) are NOT acceptable
- DROP these existing self-referential tests:
  - `score_orientation_zero_iterations_matches_raw_score` — consistency, not ground truth
  - `cube_metrics_match_score_components` — consistency, not ground truth
  - All "temp candidate ≈ candidate" tests in compute.test.ts
  - Determinism tests (necessary but not correctness proofs — keep as separate "determinism" category)

### WASM API Design (LOCKED)
New WASM exports:
- `score_all_directions(positions, normals, areas, directions, critical_angle, refine_iters, exclude_unstable) -> Float32Array` — scores ALL directions, returns N×12 floats per direction [qx, qy, qz, qw, overhang, footprint, cross, surface, height, stable, margin, contact_area]
- `rank_candidates(metrics_flat, weights, method) -> Float32Array` — ranks by method ("weights"/"consensus"/"topsis"), returns N×2 [index, composite_score] sorted
- `compute_norm_bounds(positions, normals, areas, directions, critical_angle) -> Float32Array` — samples ~30 directions, returns [lo[5], hi[5]] normalization bounds

### TS Thin Layer (LOCKED)
- Keep in TS: `OriData`, `Candidate`, `ComputeConfig`, `SliceResult` type definitions (as WASM return shape definitions)
- Keep in TS: `WEIGHT_PRESETS` + `loadProfiles` — weight config passed TO WASM, not used for calculation
- Keep in TS: `decimateForScore` — or move to Rust (planner decides)
- Delete from TS: ALL metric functions, ALL ranking functions, ALL selection/yaw/stability functions, ALL geometry helpers (convexHull2D, polygonArea, etc.)
- Worker (`orient.worker.ts`): single call to WASM `score_all_directions`, post back results directly

### Surface Quality Formula (LOCKED)
- `misalignment_score` is a BENEFIT metric: HIGHER = better
- Normalized cost form is `(sH - surf) / sSpan` (NOT `(surf - sL) / sSpan`)
- This must be preserved in the Rust ranking implementations

### Existing Rust Code to Reuse (LOCKED)
- `core/src/scoring.rs` — already has `score_candidate`, `footprint_area`, `max_cross_section`, `misalignment_score`, `min_z_height`, `shadowed_overhang_fraction`, `score_components`
- `core/src/stability.rs` — already has `check_stability`, `convex_hull_2d`, `polygon_area`, `point_in_convex_polygon`, `min_edge_distance`
- `core/src/lib.rs` — already has `score_orientation`, `refine_orientation`, `refine_orientation_batch`
- These are the single source of truth — TS duplicates are deleted, not the other way around

### the agent's Discretion
- Whether to move `decimateForScore` to Rust or keep in TS (it's a sampling function, not a metric)
- How to structure the Rust ranking module (one file vs multiple)
- Whether to keep the worker as a thin WASM dispatcher or eliminate it entirely
- How to handle progress callbacks from WASM (JS callback function vs polling)
- Exact CLI output format (JSON schema details)
</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementation.**

### Current Architecture (What's Being Replaced)
- `web/src/compute.ts` — ALL TS metric/ranking implementations being deleted (989 lines)
- `web/src/compute.test.ts` — ALL TS tests being deleted (614 lines)
- `web/src/main.ts` — UI orchestration being updated to call WASM instead of TS metrics
- `web/src/orient.worker.ts` — worker being simplified or eliminated
- `core/src/scoring.rs` — Rust metrics (single source of truth, being kept)
- `core/src/stability.rs` — Rust stability (being kept, exposed via WASM)
- `core/src/lib.rs` — WASM exports (being extended with new functions)
- `core/Cargo.toml` — crate config (adding `[[bin]]` section for CLI)

### Project Context
- `.planning/ROADMAP.md` — Phase 5 definition and success criteria
- `.planning/STATE.md` — project state and history
- `.planning/REQUIREMENTS.md` — original requirements
- `.opencode/skills/spike-findings-orient-stl/SKILL.md` — spike findings (implementation patterns, constraints, gotchas)
</canonical_refs>

<specifics>
## Specific Ideas

### Current Duplication Map (TS → Rust)
| TS Function (compute.ts) | Rust Function (scoring.rs/stability.rs) | Status |
|---------------------------|-----------------------------------------|--------|
| `scoreCandidate` | `score_candidate` | Duplicate — delete TS |
| `footprintArea` | `footprint_area` | Duplicate — delete TS |
| `maxCrossSection` | `max_cross_section` | Duplicate — delete TS |
| `misalignmentScore` | `misalignment_score` | Duplicate — delete TS |
| `shadowedOverhangFraction` | `shadowed_overhang_fraction` | Duplicate — delete TS |
| `computeHeight` | `min_z_height` | Duplicate — delete TS |
| `checkStability` | `check_stability` (stability.rs) | Duplicate — delete TS |
| `computeDefaultYaw` | (none in Rust) | TS-only — port to Rust |
| `rankByWeights` | (none in Rust) | TS-only — port to Rust |
| `rankByConsensus` | (none in Rust) | TS-only — port to Rust |
| `rankByTopsis` | (none in Rust) | TS-only — port to Rust |
| `mergeCandidates` | (none in Rust) | TS-only — port to Rust |
| `computeSlice` | (none in Rust) | TS-only — replace with WASM `score_all_directions` |
| `minShadowedOverhang` | (none in Rust) | TS-only — port to Rust or fold into `score_all_directions` |

### Ground-Truth Test Examples (Hand-Computed)
- **Overhang**: Single triangle area 0.5 at 60° from dir, critical_angle=30° → penalty = 0.5 × (cos60° - cos30°) = 0.5 × (0.5 - 0.866) — wait, cos60°=0.5 < cos30°=0.866, so NO penalty. At 20° from dir: cos20°=0.940 > cos30°=0.866 → penalty = 0.5 × (0.940 - 0.866) = 0.037
- **Footprint**: Unit square (area 1.0) face-on to dir → 1.0. At 45° → cos45°=0.707. Edge-on → 0.
- **Height**: Unit cube (0,0,0)-(1,1,1) along Y → 1.0. Flat slab at z=0 along Z → 0.
- **TOPSIS**: 3 candidates with known metrics → hand-compute vector normalization, weighted distances, closeness coefficients
- **Weighted sum**: 3 candidates → hand-compute min-max normalization, weighted scores, sort order
- **Consensus**: 3 candidates → hand-compute 1-max(normalized costs) for each
- **Yaw**: 2×1 rectangle hull → optimal yaw=0° (already minimal). L-shaped hull → known rotation.
- **Selection**: 5 directions at 0°, 10°, 20°, 30°, 40° with 15° diversity filter → keeps 0°, 20°, 40° (or similar)
</specifics>

<deferred>
## Deferred Ideas

- WASM multithreading via SharedArrayBuffer/rayon — single-threaded is sufficient for ~50-300 directions
- Web Worker offloading of WASM calls — can revisit if scoring becomes slow on large meshes
- Moving `decimateForScore` to Rust — planner decides; it's sampling, not a metric
- CLI regression test harness (diff CLI output against expected JSON) — can add after basic CLI works
</deferred>

---

*Phase: 05-rust-consolidation*
*Context gathered: 2026-07-13 via direct user discussion*
