# Phase 5: Consolidate All Calculations in Rust - Research

**Researched:** 2026-07-13
**Domain:** Rust/WASM consolidation — porting metrics, rankings, selection, and yaw from TypeScript to Rust; dual-target crate (WASM cdylib + native CLI binary); ground-truth test strategy
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Architecture: Single WASM Call (LOCKED)**
- Replace the worker-based `computeSlice` pipeline with a single WASM `score_all_directions` function that scores ALL directions in Rust
- Workers are eliminated entirely — one WASM call handles the full candidate search
- Progress callbacks via JS (WASM calls a JS callback function periodically)
- Rationale: Simpler, one source of truth, no worker coordination complexity. WASM is fast enough for single-threaded scoring of ~50-300 directions.

**CLI Structure: Binary in Same Crate (LOCKED)**
- Add `src/bin/cli.rs` to the `orient-core` crate
- Shares all code with the WASM build via the `rlib` crate type (already configured)
- Usage: `cargo run --bin cli -- input.stl --angle 30 --profile resin-biased --ranker topsis`
- Outputs JSON with all candidates, metrics, and composite scores
- Can be used for regression testing and correctness verification

**Test Strategy: Delete All TS Metric/Ranking Tests (LOCKED)**
- Remove `compute.test.ts` entirely (52 tests for TS metric/ranking duplicates)
- Keep TS test files: `quaternion.test.ts`, `rotate.test.ts`, `convention.test.ts`, `centering.test.ts` (display/rendering math)
- All metric, ranking, and selection tests live in Rust (`cargo test`)
- Ground-truth tests only — hand-computed expected values from known geometry

**Ground-Truth Test Requirements (LOCKED)**
- Every test must use a mesh with known geometry where the expected value can be computed by hand (arithmetic, not by running the implementation)
- Tests that compare an implementation to itself (consistency tests) are NOT acceptable
- DROP these existing self-referential tests:
  - `score_orientation_zero_iterations_matches_raw_score` — consistency, not ground truth
  - `cube_metrics_match_score_components` — consistency, not ground truth
  - All "temp candidate ≈ candidate" tests in compute.test.ts
  - Determinism tests (necessary but not correctness proofs — keep as separate "determinism" category)

**WASM API Design (LOCKED)**
New WASM exports:
- `score_all_directions(positions, normals, areas, directions, critical_angle, refine_iters, exclude_unstable) -> Float32Array` — scores ALL directions, returns N×12 floats per direction [qx, qy, qz, qw, overhang, footprint, cross, surface, height, stable, margin, contact_area]
- `rank_candidates(metrics_flat, weights, method) -> Float32Array` — ranks by method ("weights"/"consensus"/"topsis"), returns N×2 [index, composite_score] sorted
- `compute_norm_bounds(positions, normals, areas, directions, critical_angle) -> Float32Array` — samples ~30 directions, returns [lo[5], hi[5]] normalization bounds

**TS Thin Layer (LOCKED)**
- Keep in TS: `OriData`, `Candidate`, `ComputeConfig`, `SliceResult` type definitions (as WASM return shape definitions)
- Keep in TS: `WEIGHT_PRESETS` + `loadProfiles` — weight config passed TO WASM, not used for calculation
- Keep in TS: `decimateForScore` — or move to Rust (planner decides)
- Delete from TS: ALL metric functions, ALL ranking functions, ALL selection/yaw/stability functions, ALL geometry helpers (convexHull2D, polygonArea, etc.)
- Worker (`orient.worker.ts`): single call to WASM `score_all_directions`, post back results directly

**Surface Quality Formula (LOCKED)**
- `misalignment_score` is a BENEFIT metric: HIGHER = better
- Normalized cost form is `(sH - surf) / sSpan` (NOT `(surf - sL) / sSpan`)
- This must be preserved in the Rust ranking implementations

**Existing Rust Code to Reuse (LOCKED)**
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

### Deferred Ideas (OUT OF SCOPE)
- WASM multithreading via SharedArrayBuffer/rayon — single-threaded is sufficient for ~50-300 directions
- Web Worker offloading of WASM calls — can revisit if scoring becomes slow on large meshes
- Moving `decimateForScore` to Rust — planner decides; it's sampling, not a metric
- CLI regression test harness (diff CLI output against expected JSON) — can add after basic CLI works
</user_constraints>

## Summary

Phase 5 consolidates all computation into a single Rust source of truth. Currently, the codebase has a TS/Rust duplication: `scoring.rs` implements the 6 metrics (overhang, footprint, cross-section, surface quality, height, shadowed overhang), but `compute.ts` re-implements all of them in TypeScript plus the ranking algorithms (weighted sum, consensus, TOPSIS), candidate selection (angle-diversity merge), and yaw optimization (bbox-minimizing). The WASM boundary today is at `prepare_data()` (geometry + directions) and `score_orientation()` (per-direction refine + metrics); everything else runs in JS workers.

This phase moves everything calculation-related to Rust. Three new Rust modules are needed: **ranking** (porting `rankByWeights`, `rankByConsensus`, `rankByTopsis`), **selection** (porting `mergeCandidates` with angle-diversity), and **yaw** (porting `computeDefaultYaw` + quaternion helpers `quaternionAlign`, `multiplyQuats`). A new `score_all_directions` WASM export replaces the multi-worker `computeSlice` pipeline. A CLI binary (`src/bin/cli.rs`) using `clap` (derive API) + `serde_json` enables independent verification outside the browser.

The dual-target requirement (WASM `cdylib` + native `[[bin]]`) demands **feature-gating** the wasm-only dependencies (`wasm-bindgen`, `js-sys`, `serde-wasm-bindgen`, `console-error-panic-hook`) behind a `wasm` feature, and the CLI deps (`serde_json`, `clap`) behind a `cli` feature. `wasm-bindgen` 0.2.126 compiles on `x86_64-unknown-linux-gnu` (confirmed via docs.rs build), so the core scoring modules compile on both targets; only the `#[wasm_bindgen]` entry-point functions need `#[cfg(feature = "wasm")]` gates.

The ground-truth test strategy requires hand-computed expected values from known mesh geometry. The existing `scoring.rs` and `stability.rs` tests are already ground-truth (unit square → area 1.0, 45° → cos(45°)=√2/2, etc.). The new ranking, selection, and yaw tests must follow the same pattern: tiny meshes with 2-3 candidates where the expected TOPSIS closeness, weighted-sum sort order, or angle-diversity selection can be computed by hand.

**Primary recommendation:** Create three new Rust modules (`ranking.rs`, `selection.rs`, `yaw.rs`), add `score_all_directions` / `rank_candidates` / `compute_norm_bounds` to `lib.rs` (feature-gated), add a `cli.rs` binary with `clap` + `serde_json`, feature-gate the wasm-only deps, delete all TS metric/ranking/selection/yaw code, keep a single thin worker as WASM dispatcher, and write ground-truth tests for every new Rust function.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Per-direction metric scoring (H1-H6, H11) | Rust core (scoring.rs) | — | Already there; single source of truth. TS deletes duplicates. |
| Hill-climb refinement (xorshift32 seeded) | Rust core (lib.rs refine_once) | — | Already there; stays. |
| Ranking (weighted sum, consensus, TOPSIS) | Rust core (NEW ranking.rs) | — | Currently TS-only in `compute.ts`. Port to Rust as the single implementation. |
| Candidate selection (angle-diversity merge) | Rust core (NEW selection.rs) | — | Currently TS-only `mergeCandidates`. Port to Rust. |
| Yaw optimization (bbox-minimizing, 180 samples) | Rust core (NEW yaw.rs) | — | Currently TS-only `computeDefaultYaw`. Port to Rust. |
| Quaternion construction (align + yaw composition) | Rust core (NEW yaw.rs) | — | Currently TS-only `quaternionAlign` + `multiplyQuats`. Port to Rust. |
| Full-direction scoring pipeline | Rust core (lib.rs `score_all_directions`) | — | NEW: replaces multi-worker `computeSlice`. One WASM call for all directions. |
| WASM entry points (JS boundary) | Rust lib.rs `#[wasm_bindgen]` fns | — | Feature-gated `wasm`. Existing + 3 new exports. |
| CLI binary (STL → JSON pipeline) | Rust `src/bin/cli.rs` | — | NEW binary in same crate, shared rlib. Feature-gated `cli`. |
| Decimation (mesh sampling for speed) | TS (`decimateForScore`) or Rust | — | Agent discretion. Currently TS. See Open Questions. |
| Profile weight config (JSON) | TS (`profiles/*.json` + loader) | — | Stays in TS; config passed TO WASM, not used for calculation. |
| Type definitions (OriData, Candidate, etc.) | TS | — | Stays in TS as WASM return-shape definitions. |
| Rendering, navigation, export, UI | TS (main.ts, viewport, exportSTL) | — | Unchanged — rendering-only, calls WASM for all computation. |

## Standard Stack

### Core (existing — kept)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `wasm-bindgen` | 0.2.126 (currently "0.2") | JS↔Rust FFI for WASM exports | De facto standard; already in use. Feature-gated for dual-target. [VERIFIED: docs.rs/cargo search] |
| `serde` | 1 (with derive) | Serialization for config + CLI JSON | Already in use. Non-optional (needed for both WASM config deserialize and CLI JSON output). |
| `stl_io` | 0.11 | Binary STL parsing in WASM + native | Already in use; proven WASM-compatible. [VERIFIED: spike findings] |
| `js-sys` | 0.3.103 | JS system types (Function, Array) for WASM callbacks | Feature-gated `wasm`. Already in use. |

### New (CLI binary)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `serde_json` | 1.0.150 | JSON output for CLI binary | CLI feature only. Serializes Candidate results to stdout. [VERIFIED: cargo search — 17.7M wk downloads, 2015] |
| `clap` | 4.6.1 | CLI argument parsing (derive API) | CLI feature only. Parses `--angle`, `--profile`, `--ranker`, positional STL path. [VERIFIED: cargo search — 15.3M wk downloads, 2015] |

### Supporting (existing — feature-gated)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `serde-wasm-bindgen` | 0.6 | Serde ↔ JsValue conversion in WASM | Feature-gated `wasm`. Used by `prepare_data` config parsing. |
| `console-error-panic-hook` | 0.1 | WASM panic → console.error | Feature-gated `wasm`. Used in `init()`. |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `clap` (derive) | `std::env::args` manual parsing | clap gives `--help`, type validation, and enum ValueEnum for ranker/profile for ~100 lines less boilerplate. Worth the dep. |
| `serde_json` manual formatting | `serde_yaml` / manual `println!` | JSON is the natural format for structured candidate data; serde_json is zero-config with derive. |
| `js_sys::Function` callback | `Closure::wrap` + `wasm-bindgen` closure | For progress reporting, `js_sys::Function` is simpler (pass a JS fn, call `.call1`). Closures are for long-lived Rust→JS callbacks. |

**Installation (Cargo.toml changes):**
```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
stl_io = "0.11"

# WASM-only — feature-gated
wasm-bindgen = { version = "0.2", optional = true }
serde-wasm-bindgen = { version = "0.6", optional = true }
js-sys = { version = "0.3.103", optional = true }
console-error-panic-hook = { version = "0.1", optional = true }

# CLI-only — feature-gated
serde_json = { version = "1", optional = true }
clap = { version = "4", features = ["derive"], optional = true }

[features]
default = ["wasm"]
wasm = ["wasm-bindgen", "serde-wasm-bindgen", "js-sys", "console-error-panic-hook"]
cli = ["serde_json", "clap"]

[[bin]]
name = "cli"
path = "src/bin/cli.rs"
required-features = ["cli"]
```

**Build commands:**
```bash
# WASM (default features include wasm):
wasm-pack build core --target bundler --out-dir web/pkg

# CLI binary:
cargo run --bin cli --features cli -- input.stl --angle 30 --profile resin-biased --ranker topsis

# Rust tests (no features needed — core modules have no wasm deps):
cargo test
```

**Version verification (run 2026-07-13):**
```bash
cargo search serde_json  →  serde_json = "1.0.150"
cargo search clap        →  clap = "4.6.1"
# wasm-bindgen 0.2.126 confirmed via docs.rs (builds x86_64-unknown-linux-gnu)
```

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| `clap` | crates.io | 11 yrs (2015) | 15.3M/wk | github.com/clap-rs/clap | OK | Approved |
| `serde_json` | crates.io | 11 yrs (2015) | 17.7M/wk | github.com/serde-rs/json | OK | Approved |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
                         ┌──────────────────────────────────────────────────┐
                         │              Rust `orient-core` crate             │
                         │                                                  │
    ┌─────────┐ bytes    │  ┌─────────┐  ┌─────────┐  ┌──────────┐          │
    │ STL file│─────────▶│  │ stl.rs  │─▶│ mesh.rs │─▶│ hull.rs  │          │
    └─────────┘          │  └─────────┘  └─────────┘  └────┬─────┘          │
                         │      stl_io      precompute      │                │
                         │                                   ▼                │
                         │  ┌───────────┐  ┌──────┐  ┌───────────────┐      │
                         │  │candidates │─▶│decim.│  │  scoring.rs   │      │
                         │  └─────┬─────┘  └──────┘  │  stability.rs │      │
                         │        │                  └───────┬───────┘      │
                         │        │                         │ metrics      │
                         │        ▼                         ▼              │
                         │  ┌──────────┐  ┌──────────┐  ┌──────────┐       │
                         │  │  yaw.rs  │  │selection │◀─│ranking.rs│       │
    ┌──────────┐ result  │  │(NEW)     │  │.rs(NEW) │  │(NEW)     │       │
    │  stdout  │◀────────│  └────┬─────┘  └────┬─────┘  └──────────┘       │
    │  (JSON)  │         │       │quat    │diverse    scores             │
    └──────────┘         │  ┌────▼─────────▼────────────────────┐          │
         CLI binary      │  │        lib.rs  orchestration       │          │
                         │  │  score_all_directions()  [wasm]    │          │
                         │  │  rank_candidates()       [wasm]    │          │
                         │  │  compute_norm_bounds()   [wasm]    │          │
                         │  │  prepare_data(), refine*  [wasm]   │          │
                         │  │  CLI pipeline             [cli]    │          │
                         │  └───────────┬───────────────────────┘          │
                         └──────────────┼──────────────────────────────────┘
                                        │ WASM FFI (Float32Array)
                                        ▼
                         ┌──────────────────────────────────────────────────┐
                         │              TypeScript thin layer                │
                         │                                                  │
                         │  orient.worker.ts ──▶ wasm.score_all_directions  │
                         │       ▲                    │                     │
                         │       │ postMessage        │ progress callback   │
                         │       │ (results)           │ (js_sys::Function)  │
                         │  ┌────┴──────┐         ┌────▼──────────────┐     │
                         │  │  main.ts   │◀────────│ one thin worker   │     │
                         │  │  (render)  │  results │ (WASM dispatcher) │     │
                         │  └────┬──────┘         └───────────────────┘     │
                         │       │                                          │
                         │  ┌────▼──────┐  ┌──────────┐  ┌───────────┐     │
                         │  │ viewport   │  │ exportSTL│  │ profiles/ │     │
                         │  │ three.js   │  │          │  │  (JSON)   │     │
                         │  └───────────┘  └──────────┘  └───────────┘     │
                         └──────────────────────────────────────────────────┘
```

**Data flow:** STL bytes → Rust parse (stl_io) → mesh precompute (normals/areas/centroids) → hull → candidate directions → `score_all_directions` (all metrics + stability + yaw + quaternion per direction) → `rank_candidates` (weighted/consensus/TOPSIS) → selection (angle-diversity merge) → results returned to TS → rendering only.

### Recommended Project Structure
```
core/
├── Cargo.toml          # ADD: [[bin]] section, feature-gate wasm deps, add cli deps
├── src/
│   ├── lib.rs          # EXTEND: score_all_directions, rank_candidates, compute_norm_bounds (feature-gated wasm); CLI pipeline (feature-gated cli)
│   ├── stl.rs          # KEEP: parse_stl (shared by WASM + CLI)
│   ├── mesh.rs         # KEEP: precompute_mesh (shared)
│   ├── hull.rs         # KEEP: compute_hull (shared)
│   ├── candidates.rs   # KEEP: generate_candidates, deduplicate_directions (shared)
│   ├── decimate.rs     # KEEP: sample_for_hull (shared)
│   ├── rng.rs          # KEEP: xorshift32 PRNG (shared)
│   ├── scoring.rs      # KEEP: all 6 metrics + score_components + shadowed_overhang_fraction
│   ├── stability.rs    # KEEP: check_stability + 2D geometry helpers
│   ├── ranking.rs      # NEW: rank_by_weights, rank_by_consensus, rank_by_topsis
│   ├── selection.rs    # NEW: merge_candidates (angle-diversity), angle_between
│   ├── yaw.rs          # NEW: compute_default_yaw, quaternion_align, multiply_quats, perpendicular_basis (yaw-specific)
│   ├── harness.rs      # KEEP: integration test harness (ignored tests)
│   └── bin/
│       └── cli.rs      # NEW: CLI binary — clap derive, reads STL, runs pipeline, outputs JSON
web/
├── src/
│   ├── compute.ts      # STRIP: delete ALL metrics/rankings/selection/yaw/geometry; KEEP type defs only
│   ├── compute.test.ts # DELETE entirely
│   ├── orient.worker.ts # SIMPLIFY: single WASM dispatcher (score_all_directions)
│   ├── main.ts         # UPDATE: call WASM functions instead of TS compute functions
│   ├── profiles/        # KEEP: JSON weight configs + loader
│   └── quaternion.test.ts, rotate.test.ts, convention.test.ts, centering.test.ts  # KEEP (rendering math tests)
```

### Pattern 1: Feature-Gating for Dual-Target Crate
**What:** The crate must build as `cdylib` (WASM via wasm-pack) AND as a native binary (CLI via cargo). The wasm-only deps are feature-gated so the CLI build doesn't pull them.
**When to use:** Any crate that serves both WASM and native targets.
**Key fact:** `wasm-bindgen` 0.2.126 compiles on `x86_64-unknown-linux-gnu` (confirmed: docs.rs builds it natively). `serde-wasm-bindgen`, `js-sys`, and `console-error-panic-hook` are wasm-specific and MUST be feature-gated. `console_error_panic_hook::set_once()` will fail on native. [VERIFIED: docs.rs platform indicator for wasm-bindgen]

```toml
# Cargo.toml — the dual-target pattern
[lib]
crate-type = ["cdylib", "rlib"]

[features]
default = ["wasm"]
wasm = ["wasm-bindgen", "serde-wasm-bindgen", "js-sys", "console-error-panic-hook"]
cli = ["serde_json", "clap"]

[[bin]]
name = "cli"
path = "src/bin/cli.rs"
required-features = ["cli"]
```

```rust
// lib.rs — gate the WASM entry points
use serde::{Deserialize, Serialize};
// Core modules compile unconditionally (no wasm deps):
mod stl; mod mesh; mod hull; mod candidates; mod decimate;
mod scoring; mod stability; mod ranking; mod selection; mod yaw;

#[cfg(feature = "wasm")]
mod wasm_exports;
#[cfg(feature = "wasm")]
pub use wasm_exports::*;  // re-export prepare_data, score_all_directions, etc.

#[cfg(feature = "cli")]
pub mod pipeline;  // CLI-accessible pipeline functions (parse → score → rank → select)
```

### Pattern 2: Progress Callbacks from WASMvia js_sys::Function
**What:** Pass a JavaScript function to WASM; Rust calls it periodically during long computation.
**When to use:** The `score_all_directions` function needs to report progress.
**Key insight:** A `js_sys::Function` is the simplest callback mechanism — no `Closure` lifecycle management, no `forget()`. [CITED: rustwasm.github.io/wasm-bindgen/examples/closures.html]

```rust
// Source: wasm-bindgen guide — closures example + js_sys::Function API
use wasm_bindgen::prelude::*;
use js_sys::Function;

#[wasm_bindgen]
pub fn score_all_directions(
    positions: &[f32],
    normals: &[f32],
    areas: &[f32],
    directions: &[f32],
    critical_angle: f32,
    refine_iters: u32,
    exclude_unstable: bool,
    progress: Option<&Function>,  // JS callback (dir_index, total_dirs) → void
) -> Vec<f32> {
    let total = directions.len() / 3;
    let mut out = Vec::with_capacity(total * 12);
    for i in 0..total {
        let dir = [directions[i*3], directions[i*3+1], directions[i*3+2]];
        // ... compute all metrics, stability, yaw, quaternion ...
        out.extend_from_slice(&[qx, qy, qz, qw, overhang, foot, cross, surf, height, stable, margin, contact]);

        if let Some(cb) = progress {
            if i % 10 == 0 {
                let _ = cb.call2(
                    &JsValue::UNDEFINED,
                    &JsValue::from_f64(i as f64),
                    &JsValue::from_f64(total as f64),
                );
            }
        }
    }
    out
}
```

### Pattern 3: CLI Binary with clap Derive API
**What:** A native binary in the same crate that parses args, runs the full pipeline, and outputs JSON.
**When to use:** Independent verification without a browser. [VERIFIED: docs.rs/clap — derive API tutorial]

```rust
// src/bin/cli.rs
use clap::Parser;
use serde::Serialize;

#[derive(Parser)]
#[command(version, about = "orient-stl CLI — independent verification")]
struct Cli {
    /// Path to binary STL file
    input: String,

    /// Critical angle in degrees (default 30)
    #[arg(long, default_value_t = 30.0)]
    angle: f32,

    /// Weight profile name (e.g., resin-biased, equal, overhang-only)
    #[arg(long, default_value = "resin-biased")]
    profile: String,

    /// Ranker method: weights, consensus, topsis
    #[arg(long, default_value = "consensus")]
    ranker: String,

    /// Refine iterations per direction (default 0)
    #[arg(long, default_value_t = 0)]
    refine: u32,

    /// Max candidates after diversity selection
    #[arg(long, default_value_t = 20)]
    max_candidates: usize,
}

#[derive(Serialize)]
struct CliCandidate {
    rank: usize,
    #[serde(rename = "compositeScore")]
    composite_score: f32,
    quaternion: [f32; 4],
    overhang: f32,
    footprint: f32,
    #[serde(rename = "maxCross")]
    max_cross: f32,
    surface: f32,
    height: f32,
    shadowed: f32,
    stable: bool,
    margin: f32,
    #[serde(rename = "contactArea")]
    contact_area: f32,
    dir: [f32; 3],
}

fn main() {
    let cli = Cli::parse();
    let bytes = std::fs::read(&cli.input).expect("Cannot read STL file");
    // ... pipeline: parse → mesh → hull → candidates → score_all → rank → select → JSON ...
    let output: Vec<CliCandidate> = /* ... */;
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
```

### Pattern 4: Ground-Truth Test Pattern
**What:** Every test uses a mesh where the expected value is computed by hand from geometry, not by running the implementation.
**When to use:** All new ranking, selection, and yaw tests. Existing scoring/stability tests are already ground-truth.

```rust
// Example: TOPSIS ground-truth test (3 candidates, 2 metrics, known weights)
#[test]
fn topsis_three_candidates_known_geometry() {
    // 3 candidates with hand-computed metrics:
    //   C0: overhang=0.0, height=1.0  (best overhang, worst height)
    //   C1: overhang=1.0, height=0.5  (worst overhang, mid height)
    //   C2: overhang=0.5, height=0.0  (mid overhang, best height)
    // weights: wOverhang=1.0, wHeight=1.0, others=0
    //
    // Vector norm: overhang column sqrt(0²+1²+0.5²)=sqrt(1.25)=1.118
    //   normalized: [0, 0.894, 0.447]
    //   weighted:   [0, 0.894, 0.447]
    // height column sqrt(1²+0.5²+0²)=sqrt(1.25)=1.118
    //   normalized: [0.894, 0.447, 0]
    //   weighted:   [0.894, 0.447, 0]
    //
    // Ideal best (cost→min, so best=lowest):
    //   overhang best=0 (C0), worst=0.894 (C1)
    //   height best=0 (C2), worst=0.894 (C0)
    //
    // S+ (dist to best): C0: sqrt(0.894²)=0.894; C1: sqrt(0.894²+0.447²)=1.0; C2: sqrt(0.447²)=0.447
    // S- (dist to worst): C0: sqrt(0²)=0;     C1: sqrt(0²+0.447²)=0.447; C2: sqrt(0.894²+0.894²)=1.265
    // C_i = S-/(S++S-):  C0: 0/(0.894+0)=0.0; C1: 0.447/(1.0+0.447)=0.309; C2: 1.265/(0.447+1.265)=0.739
    // Rank order: C2 > C1 > C0
    let candidates = vec![
        /* C0, C1, C2 with metrics above */
    ];
    let weights = ScoreWeights { w_overhang: 1.0, w_footprint: 0.0, w_cross: 0.0, w_surface: 0.0, w_height: 1.0 };
    let ranked = rank_by_topsis(&candidates, &weights);
    assert!((ranked[0].dir[0] - /* C2 */).abs() < 1e-6);
    assert!((ranked[0].composite_score - 0.739).abs() < 0.01);
    assert!((ranked[2].composite_score - 0.0).abs() < 0.01);
}
```

### Anti-Patterns to Avoid
- **Anti-pattern: Running `score_all_directions` on the main thread without a worker.** A synchronous WASM call blocks the JS event loop — the browser cannot repaint the DOM between progress callbacks, so the progress bar visually freezes. **Do instead:** Keep ONE thin worker that calls `score_all_directions` and posts progress/results back via `postMessage`. This keeps the UI thread free.
- **Anti-pattern: Using `(surf - sL) / sSpan` for surface quality cost.** Surface quality is a BENEFIT metric (higher=better). The cost form must be `(sH - surf) / sSpan`. Getting this backwards inverts the ranking. [LOCKED in CONTEXT.md]
- **Anti-pattern: Writing consistency tests (compare implementation to itself).** E.g., "rankByTopsis output matches rankByTopsis output" proves nothing. **Do instead:** Hand-compute expected values from known geometry.
- **Anti-pattern: Gate ALL of `lib.rs` behind `#[cfg(feature = "wasm")]`.** The core modules (scoring, ranking, selection, yaw, mesh, hull, candidates, stl, decimate, rng) have NO wasm deps and compile on both targets. Only the `#[wasm_bindgen]` entry-point functions and wasm-only imports need gating. The CLI binary imports the core modules directly.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CLI argument parsing | Manual `std::env::args` loop | `clap` derive API | `--help` generation, type validation, `ValueEnum` for ranker names, error messages — all free. ~10 years battle-tested. |
| JSON serialization | Manual string formatting | `serde_json` + `#[derive(Serialize)]` | Nested arrays, pretty-print, f32 precision handling — trivial with derive. Manual formatting is error-prone for quaternions + 12 floats per candidate. |
| WASM callback dispatch | `Closure::wrap` + lifecycle management | `js_sys::Function` + `.call2()` | For a simple progress callback (fire-and-forget), `Function` is 5 lines vs `Closure`'s 20+ with `forget()` memory leaks. |
| TOPSIS vector normalization | Custom sqrt-sum logic copy | Port the EXISTING TS `normCol` to Rust | The TS `rankByTopsis` is already verified correct (Phase 3.5 TDD). Port it faithfully — don't re-derive the math. |
| Yaw bbox minimization | Rotating calipers from scratch | Port the EXISTING TS `computeDefaultYaw` | The TS version uses 180-sample brute-force (not rotating calipers). Port the same algorithm — it's O(180×H) where H is hull size, fast enough. |

**Key insight:** Phase 3.5 already verified the ranking algorithms (weighted sum, consensus, TOPSIS) via TDD in TypeScript. The Rust port must be a FAITHFUL translation of the TS logic, not a re-derivation. The TS code is the reference; the Rust code is the replacement. Same formulas, same normalization, same sort order.

## Common Pitfalls

### Pitfall 1: Main-Thread Blocking During `score_all_directions`
**What goes wrong:** If `score_all_directions` is called directly from `main.ts` (no worker), the synchronous WASM call blocks the JS event loop for the entire scoring duration. Progress callbacks fire but the browser can't repaint the DOM, so the progress bar visually freezes. The UI appears hung.
**Why it happens:** WASM calls are synchronous by default — there's no `await` in the WASM→JS sync path. A single call processing 300 directions × (6 metrics + stability + refine) can take 500ms-2s on large meshes.
**How to avoid:** Keep ONE thin worker (`orient.worker.ts`) that calls `score_all_directions` and posts results/progress back via `postMessage`. The main thread receives messages and updates the DOM freely. This eliminates multi-worker COORDINATION complexity (the locked decision's intent) while keeping the UI responsive.
**Warning signs:** Progress bar frozen at 0% then jumping to 100%; "Page Unresponsive" browser warning on large meshes.

### Pitfall 2: Surface Quality Formula Inversion
**What goes wrong:** Ranking produces inverted results — candidates with worse surface quality score higher.
**Why it happens:** `misalignment_score` is a BENEFIT metric (higher=better). The cost normalization must be `(sH - surf) / sSpan`, NOT `(surf - sL) / sSpan`. The TS `rankByWeights` and `rankByConsensus` both invert it correctly (line 809: `const sn = (sMax - c.surfaceQuality) / sSpan`), but `rankByConsensus` uses `invert()` for surface (line 850). A careless port can swap the direction.
**How to avoid:** Add a ground-truth test: 2 candidates where C0 has surface=1.0 and C1 has surface=2.0. With weights `{wSurface: 1.0}` (cost), C1 must rank HIGHER. The expected order is C1 > C0. [LOCKED: CONTEXT.md Surface Quality Formula]
**Warning signs:** TOPSIS produces the exact reverse order of weighted-sum for surface-heavy profiles.

### Pitfall 3: `console_error_panic_hook::set_once()` on Native Build
**What goes wrong:** `cargo run --bin cli` fails to compile because `console_error_panic_hook` is not available or has no effect on native targets.
**Why it happens:** `console_error_panic_hook` installs a panic hook that calls `console.error` — a JS API. It has no native implementation.
**How to avoid:** Feature-gate `console_error_panic_hook` behind the `wasm` feature. The `init()` function should be `#[cfg(feature = "wasm")]`. The CLI binary uses Rust's default panic handler (prints to stderr).
**Warning signs:** `error[E0425]: cannot find value 'console_error_panic_hook'` during `cargo build --bin cli`.

### Pitfall 4: `serde_wasm_bindgen::from_value` on Native Build
**What goes wrong:** `prepare_data` uses `serde_wasm_bindgen::from_value` to deserialize the config `JsValue`. On native, `JsValue` doesn't exist (or is a stub).
**Why it happens:** `serde-wasm-bindgen` depends on `js-sys` and `wasm-bindgen` — wasm-only.
**How to avoid:** Feature-gate `serde-wasm-bindgen` behind `wasm`. The CLI binary parses config from `clap` args, not from `JsValue`. The `prepare_data` function is `#[cfg(feature = "wasm")]`.
**Warning signs:** Link errors or missing symbols when building `--bin cli`.

### Pitfall 5: WASM Binary Stale After Rust Edits
**What goes wrong:** Browser shows old behavior after editing Rust code.
**Why it happens:** The prebuilt `.wasm` at `web/pkg/` does NOT auto-sync with Rust source. `wasm-pack build` must be run manually after every Rust edit. [VERIFIED: spike-findings-orient-stl hard rule]
**How to avoid:** Every Rust edit task MUST include `wasm-pack build core --target bundler --out-dir web/pkg` as a verification step. The `wasm-pack` binary IS installed (`0.15.0`).
**Warning signs:** "Unknown mode", missing exports, wrong function signatures at runtime.

### Pitfall 6: Quaternion Convention Mismatch
**What goes wrong:** The yaw quaternion in Rust produces a different orientation than the TS version.
**Why it happens:** The TS `computeDefaultYaw` returns `[cos(half), dn[0]*sin(half), dn[1]*sin(half), dn[2]*sin(half)]` — a rotation by `bestAngle` around axis `dn`. The quaternion is then composed as `qFull = multiplyQuats(qYaw, qAlign)` where `qAlign = quaternionAlign(dir, [0,-1,0])`. The order matters: `qYaw * qAlign` means "first align dir to -Y, then yaw around the build axis." If the Rust port swaps the multiplication order, the orientation is wrong.
**How to avoid:** Port `multiplyQuats` EXACTLY (the TS formula is at line 567-577 of compute.ts). Add a ground-truth test: dir=[0,0,-1] (already aligned with -Y), qAlign=identity, qYaw=bbox-min → expected quaternion matches `computeDefaultYaw` output for a unit cube.

## Code Examples

### Example 1: `yaw.rs` — Porting `computeDefaultYaw`
```rust
// Source: Faithful port of web/src/compute.ts:221-256 (computeDefaultYaw)
use crate::mesh::MeshData;

/// Quaternion that minimizes the XY bounding box of the mesh when oriented
/// along `dir`. Searches 180 yaw angles, picks the one with smallest bbox area.
/// Returns [w, x, y, z] quaternion for the yaw rotation around `dir`.
pub(crate) fn compute_default_yaw(direction: &[f32; 3], mesh: &MeshData) -> [f32; 4] {
    let dl = (direction[0]*direction[0] + direction[1]*direction[1] + direction[2]*direction[2]).sqrt();
    if dl < 1e-8 { return [1.0, 0.0, 0.0, 0.0]; }
    let dn = [direction[0]/dl, direction[1]/dl, direction[2]/dl];
    let up = [-dn[0], -dn[1], -dn[2]];

    // Perpendicular basis to `up` (same logic as stability.rs find_perpendicular)
    let (up_x, up_y) = find_perpendicular(up);

    // Project all vertices to 2D (u,v) plane perpendicular to dir
    let mut pts2d: Vec<[f32; 2]> = Vec::with_capacity(mesh.vertices.len());
    for v in &mesh.vertices {
        pts2d.push([
            v[0]*up_x[0] + v[1]*up_x[1] + v[2]*up_x[2],
            v[0]*up_y[0] + v[1]*up_y[1] + v[2]*up_y[2],
        ]);
    }
    // 2D convex hull of the projection
    let hull = convex_hull_2d(&pts2d);

    let mut best_angle = 0.0f32;
    let mut best_area = f32::INFINITY;
    for s in 0..180 {
        let angle = (s as f32 / 180.0) * std::f32::consts::PI;
        let (ca, sa) = angle.sin_cos();
        let mut min_x = f32::INFINITY; let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY; let mut max_y = f32::NEG_INFINITY;
        for &[px, py] in &hull {
            let rx = px * ca - py * sa;
            let ry = px * sa + py * ca;
            if rx < min_x { min_x = rx; } if rx > max_x { max_x = rx; }
            if ry < min_y { min_y = ry; } if ry > max_y { max_y = ry; }
        }
        let area = (max_x - min_x) * (max_y - min_y);
        if area < best_area { best_area = area; best_angle = angle; }
    }
    let half = best_angle / 2.0;
    let (ch, sh) = half.cos(); // cos(half)
    let sh = sh; // sin(half)
    [ch, dn[0]*sh, dn[1]*sh, dn[2]*sh]
}

/// Quaternion that rotates vector `a` to align with vector `b` (both assumed unit).
pub(crate) fn quaternion_align(a: &[f32; 3], b: &[f32; 3]) -> [f32; 4] {
    let dot = a[0]*b[0] + a[1]*b[1] + a[2]*b[2];
    if dot > 0.9999 { return [1.0, 0.0, 0.0, 0.0]; }
    if dot < -0.9999 {
        // 180° rotation around perpendicular axis
        let axis = if a[0].abs() < 0.9 {
            [a[1]*0.0 - a[2]*1.0, a[2]*0.0 - a[0]*0.0, a[0]*1.0 - a[1]*0.0] // cross(a, [1,0,0])
        } else {
            [a[1]*0.0 - a[2]*0.0, a[2]*1.0 - a[0]*0.0, a[0]*0.0 - a[1]*1.0] // cross(a, [0,1,0])
        };
        let al = (axis[0]*axis[0] + axis[1]*axis[1] + axis[2]*axis[2]).sqrt().max(1e-12);
        return [0.0, axis[0]/al, axis[1]/al, axis[2]/al];
    }
    let axis = [
        a[1]*b[2] - a[2]*b[1],
        a[2]*b[0] - a[0]*b[2],
        a[0]*b[1] - a[1]*b[0],
    ]; // cross(a, b)
    let al = (axis[0]*axis[0] + axis[1]*axis[1] + axis[2]*axis[2]).sqrt().max(1e-12);
    let naxis = [axis[0]/al, axis[1]/al, axis[2]/al];
    let half = dot.acos() / 2.0;
    let s = half.sin();
    [half.cos(), naxis[0]*s, naxis[1]*s, naxis[2]*s]
}

/// Hamilton product: a * b (applies b first, then a).
pub(crate) fn multiply_quats(a: &[f32; 4], b: &[f32; 4]) -> [f32; 4] {
    [
        a[0]*b[0] - a[1]*b[1] - a[2]*b[2] - a[3]*b[3],
        a[0]*b[1] + a[1]*b[0] + a[2]*b[3] - a[3]*b[2],
        a[0]*b[2] - a[1]*b[3] + a[2]*b[0] + a[3]*b[1],
        a[0]*b[3] + a[1]*b[2] - a[2]*b[1] + a[3]*b[0],
    ]
}
```

### Example 2: `ranking.rs` — Porting TOPSIS
```rust
// Source: Faithful port of web/src/compute.ts:870-948 (rankByTopsis)
// Key: surfaceQuality is a BENEFIT metric → ideal-best = MAX, not min

pub(crate) struct Weights {
    pub w_overhang: f32, pub w_footprint: f32, pub w_cross: f32,
    pub w_surface: f32, pub w_height: f32,
}

pub(crate) struct CandidateMetrics {
    pub overhang: f32,      // refinedOverhang — cost (lower=better)
    pub footprint: f32,     // cost
    pub max_cross: f32,     // cost
    pub surface: f32,       // surface_quality — BENEFIT (higher=better)
    pub height: f32,        // cost
}

/// TOPSIS MCDA: vector-normalise 5 metrics, weight, compute Euclidean
/// distance to ideal-best/worst, rank by closeness C_i = S-/(S+ + S-).
pub(crate) fn rank_by_topsis(metrics: &[CandidateMetrics], w: &Weights) -> Vec<(usize, f32)> {
    let n = metrics.len();
    if n == 0 { return vec![]; }

    // Vector normalisation: v_j = x_ij / sqrt(sum(x_kj^2))
    let norm_col = |extract: fn(&CandidateMetrics) -> f32| -> Vec<f32> {
        let sq: f32 = metrics.iter().map(|c| { let v = extract(c); v * v }).sum();
        let d = sq.sqrt().max(1e-9);
        metrics.iter().map(|c| extract(c) / d).collect()
    };
    let o_n = norm_col(|c| c.overhang);
    let f_n = norm_col(|c| c.footprint);
    let c_n = norm_col(|c| c.max_cross);
    let s_n = norm_col(|c| c.surface);  // BENEFIT
    let h_n = norm_col(|c| c.height);

    // Apply weights
    let wo: Vec<f32> = o_n.iter().map(|v| v * w.w_overhang).collect();
    let wf: Vec<f32> = f_n.iter().map(|v| v * w.w_footprint).collect();
    let wc: Vec<f32> = c_n.iter().map(|v| v * w.w_cross).collect();
    let ws: Vec<f32> = s_n.iter().map(|v| v * w.w_surface).collect();
    let wh: Vec<f32> = h_n.iter().map(|v| v * w.w_height).collect();

    // Ideal-best: min for cost metrics, MAX for surface (benefit)
    let mut o_best = f32::INFINITY; let mut o_worst = f32::NEG_INFINITY;
    let mut f_best = f32::INFINITY; let mut f_worst = f32::NEG_INFINITY;
    let mut c_best = f32::INFINITY; let mut c_worst = f32::NEG_INFINITY;
    let mut s_best = f32::NEG_INFINITY; let mut s_worst = f32::INFINITY;  // inverted!
    let mut h_best = f32::INFINITY; let mut h_worst = f32::NEG_INFINITY;
    for i in 0..n {
        if wo[i] < o_best { o_best = wo[i]; } if wo[i] > o_worst { o_worst = wo[i]; }
        if wf[i] < f_best { f_best = wf[i]; } if wf[i] > f_worst { f_worst = wf[i]; }
        if wc[i] < c_best { c_best = wc[i]; } if wc[i] > c_worst { c_worst = wc[i]; }
        if ws[i] > s_best { s_best = ws[i]; } if ws[i] < s_worst { s_worst = ws[i]; }
        if wh[i] < h_best { h_best = wh[i]; } if wh[i] > h_worst { h_worst = wh[i]; }
    }

    // S+ and S- per candidate
    let mut scores: Vec<(usize, f32)> = (0..n).map(|i| {
        let mut s_plus = 0.0f32; let mut s_minus = 0.0f32;
        if w.w_overhang > 0.0 { let d = wo[i] - o_best; s_plus += d*d; let dw = o_worst - wo[i]; s_minus += dw*dw; }
        if w.w_footprint > 0.0 { let d = wf[i] - f_best; s_plus += d*d; let dw = f_worst - wf[i]; s_minus += dw*dw; }
        if w.w_cross > 0.0 { let d = wc[i] - c_best; s_plus += d*d; let dw = c_worst - wc[i]; s_minus += dw*dw; }
        if w.w_surface > 0.0 { let d = s_best - ws[i]; s_plus += d*d; let dw = ws[i] - s_worst; s_minus += dw*dw; }  // benefit: inverted
        if w.w_height > 0.0 { let d = wh[i] - h_best; s_plus += d*d; let dw = h_worst - wh[i]; s_minus += dw*dw; }
        let s_plus = s_plus.sqrt();
        let s_minus = s_minus.sqrt();
        let closeness = if s_plus + s_minus < 1e-12 { 1.0 } else { s_minus / (s_plus + s_minus) };
        (i, closeness)
    }).collect();

    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scores
}
```

### Example 3: `selection.rs` — Angle-Diversity Merge
```rust
// Source: Faithful port of web/src/compute.ts:657-759 (mergeCandidates)
// The selection loop: sort by composite score, iterate, keep candidates
// whose direction is >= minAngle away from all previously picked.

pub(crate) fn merge_candidates(
    scored: &[(usize, f32)],       // (candidate_index, composite_score) sorted
    directions: &[[f32; 3]],
    metrics: &[CandidateMetrics],
    stable_flags: &[bool],
    exclude_unstable: bool,
    max_candidates: usize,
    min_angle_deg: f32,            // typically 15.0
) -> Vec<usize> {
    let cos_threshold = (min_angle_deg * std::f32::consts::PI / 180.0).cos();
    let mut picked: Vec<[f32; 3]> = Vec::new();
    let mut result: Vec<usize> = Vec::new();
    for &(idx, _score) in scored {
        if exclude_unstable && !stable_flags[idx] { continue; }
        let dir = directions[idx];
        let too_close = picked.iter().any(|p| {
            let dot = dir[0]*p[0] + dir[1]*p[1] + dir[2]*p[2];
            dot >= cos_threshold
        });
        if !too_close {
            result.push(idx);
            picked.push(dir);
            if result.len() >= max_candidates { break; }
        }
    }
    result
}
```

### Example 4: `score_all_directions` in lib.rs (WASM export)
```rust
// Source: New function per WASM API Design (LOCKED)
// Returns N×12 floats per direction: [qx, qy, qz, qw, overhang, footprint,
// cross, surface, height, stable, margin, contact_area]

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn score_all_directions(
    positions: &[f32],
    normals: &[f32],
    areas: &[f32],
    directions: &[f32],
    critical_angle: f32,
    refine_iters: u32,
    exclude_unstable: bool,
    progress: Option<&js_sys::Function>,
) -> Vec<f32> {
    let mesh = reconstruct_mesh(positions, normals, areas);
    let hull_verts = decimate::sample_for_hull(&mesh.vertices);
    let hull = hull::compute_hull(&hull_verts);
    let total = directions.len() / 3;
    let mut out = Vec::with_capacity(total * 12);

    for i in 0..total {
        let dir = [directions[i*3], directions[i*3+1], directions[i*3+2]];
        let (dir_n, _) = normalise_dir(dir);

        // Refine (optional), then compute all 5 metrics for refined direction
        let rng = rng::Rng::new(rng::seed_from_direction(&dir_n, 0));
        let (best_dir, _) = refine_once(&mesh, &dir_n, critical_angle, refine_iters.min(500), rng);
        let c = scoring::score_components(&best_dir, &mesh, critical_angle, 64);
        let shadowed = scoring::shadowed_overhang_fraction(&best_dir, &mesh, critical_angle, 32, 0.02);

        // Stability
        let stab = stability::check_stability(&best_dir, &mesh, &hull);

        // Yaw + quaternion: qFull = qYaw * qAlign(dir, -Y)
        let q_yaw = yaw::compute_default_yaw(&best_dir, &mesh);
        let q_align = yaw::quaternion_align(&best_dir, &[0.0, -1.0, 0.0]);
        let q_full = yaw::multiply_quats(&q_yaw, &q_align);

        let stable_f = if stab.stable { 1.0f32 } else { 0.0f32 };
        out.extend_from_slice(&[
            q_full[0], q_full[1], q_full[2], q_full[3],     // quaternion [w,x,y,z]
            c.overhang, c.footprint, c.max_cross, c.surface_quality, c.height,
            stable_f, stab.margin, stab.contact_area,
        ]);

        if let Some(cb) = progress {
            if i % 10 == 0 {
                let _ = cb.call2(
                    &wasm_bindgen::JsValue::UNDEFINED,
                    &wasm_bindgen::JsValue::from_f64(i as f64),
                    &wasm_bindgen::JsValue::from_f64(total as f64),
                );
            }
        }
    }
    out
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Multi-worker `computeSlice` splitting directions across N workers | Single WASM `score_all_directions` call | Phase 5 (this phase) | Eliminates worker coordination complexity; one source of truth for scoring |
| TS metric/ranking/selection duplicates | Rust-only implementations | Phase 5 | No more drift between TS and Rust metrics; CLI verifies correctness independently |
| Self-referential tests (consistency) | Ground-truth tests (hand-computed) | Phase 5 | Tests prove correctness, not just determinism |
| `compute.ts` = 989 lines of calculation | `compute.ts` = type definitions only | Phase 5 | TS becomes rendering-only; all logic in Rust |

**Deprecated/outdated:**
- `web/src/compute.ts` metric functions (`scoreCandidate`, `footprintArea`, `maxCrossSection`, `misalignmentScore`, `shadowedOverhangFraction`, `computeHeight`, `checkStability`, `minShadowedOverhang`): all have Rust equivalents in `scoring.rs`/`stability.rs` — DELETE the TS versions
- `web/src/compute.ts` ranking functions (`rankByWeights`, `rankByConsensus`, `rankByTopsis`): port to Rust `ranking.rs` — DELETE the TS versions
- `web/src/compute.ts` selection/yaw functions (`mergeCandidates`, `computeDefaultYaw`, `computeSlice`, `quaternionAlign`, `multiplyQuats`, `convexHull2D`, `polygonArea`, `pointInConvexPolygon`, `minEdgeDistance`, `nearestCandidateScore`): port to Rust — DELETE the TS versions

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `wasm-bindgen` 0.2.126 compiles on `x86_64-unknown-linux-gnu` (can stay as non-optional dep) | Architecture Patterns | docs.rs confirms it builds natively, but `#[wasm_bindgen]` macro output may differ. If it fails, feature-gate it too — the current Cargo.toml has it as non-optional, so the minimal change is to make it optional under `wasm`. |
| A2 | 180-sample brute-force yaw (TS `computeDefaultYaw`) is fast enough in Rust WASM for ~300 directions | Architecture Patterns | 180 samples × hull size per direction × 300 directions. Hull is small (~hull face count). Should be <100ms. If slow, reduce to 90 samples (2° resolution) or precompute hull. Low risk. |
| A3 | A single thin worker is sufficient for progress reporting (vs completely eliminating workers) | Architecture Patterns Pitfall 1 | The Locked decision says "Workers are eliminated entirely" but agent discretion allows keeping a thin dispatcher. If the user insists on no workers, the progress bar shows indeterminate. Low risk — planner should keep one worker. |
| A4 | The `nearestCandidateScore` function in compute.ts (used by overlay live-score) is NOT needed in Rust | Architecture Patterns | It's a display-layer lookup (find nearest precomputed candidate by quaternion direction). It can stay in TS as it's rendering logic, not calculation. If it must move, it's a simple dot-product max. Low risk. |
| A5 | `decimateForScore` (TS mesh sampling) can stay in TS | Architecture Patterns | It's a sampling function, not a metric. The WASM `score_all_directions` receives pre-decimated data. Moving it to Rust would require an extra WASM call. Low risk — keep in TS unless planner decides otherwise. |
| A6 | `score_orientation` (existing WASM export, used by overlay live-score in main.ts) can be kept alongside new `score_all_directions` | Architecture Patterns | The overlay live-score uses `score_orientation` for per-direction refine+metrics. Keeping it avoids breaking the overlay. It already works and is ground-truth-tested. Low risk. |
| A7 | The `minShadowedOverhang` (8-sample yaw rotation for shadow minimization) should be folded into `score_all_directions` rather than being a separate TS call | Architecture Patterns | The TS `computeSlice` calls `minShadowedOverhang` per direction. In Rust, this becomes part of the per-direction scoring loop inside `score_all_directions`. The function `shadowed_overhang_fraction` already exists in Rust `scoring.rs`. Just call it 8 times with rotated basis. Low risk. |

## Open Questions (RESOLVED)

1. **Should `decimateForScore` move to Rust?**
   - RESOLVED: Keep `decimateForScore` in TS for the browser path. The CLI uses the full mesh (no decimation needed — correctness > speed for verification). Plan 03 T1 retains it in the stripped compute.ts.

2. **Should `directions` be computed in Rust for the CLI, or passed in?**
   - RESOLVED: Add a `pub fn prepare_data_native(bytes, mode, dedupe_angle) -> (MeshData, Vec<[f32;3]>)` function (feature-gated `cli`). The CLI calls it; the WASM path calls the same underlying functions via `prepare_data`. Plan 02 T2 implements this.

3. **How many ground-truth tests are needed for the new modules?**
   - RESOLVED: Write unit tests for each new function (ground-truth, tiny meshes). Plan 01 T2 implements 17+ ground-truth tests covering all ranking/selection/yaw functions. No self-referential tests.

4. **Should the existing `harness.rs` test harness be updated?**
   - RESOLVED: Update harness.rs to use the new ranking module — it becomes a CLI-adjacent integration test that validates the full pipeline on real STLs. Plan 02 T2 handles this.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo` | CLI build + Rust tests | ✓ | 1.97.0 | — |
| `rustc` | Rust compilation | ✓ | 1.97.0 | — |
| `wasm-pack` | WASM rebuild | ✓ | 0.15.0 | — |
| `wasm32-unknown-unknown` target | WASM cdylib build | ✓ | installed | — |
| `x86_64-unknown-linux-gnu` target | CLI binary build | ✓ | host target | — |
| `node` / `npm` | TS build + test runner | ✓ | 24.11.0 / 11.6.2 | — |
| `test-tetrahedron.stl` | Ground-truth test fixtures | ✓ | in repo root | — |
| `resources/Skulled_Wurm_Bird_WOBase.stl` | Integration test fixture | ✓ | in repo | — |
| `serde_json` crate | CLI JSON output | ✗ (not in Cargo.lock) | 1.0.150 | Add to Cargo.toml |
| `clap` crate | CLI arg parsing | ✗ (not in Cargo.lock) | 4.6.1 | Add to Cargo.toml |

**Missing dependencies with no fallback:** none — both new crates install fine via cargo (already verified via `cargo search`).

**Missing dependencies with fallback:** none.

## Security Domain

> This phase is an architecture consolidation (porting TS logic to Rust, adding a CLI binary).
> No new authentication, session management, access control, input validation, or cryptography domains are introduced.
> The CLI binary reads local files (user-supplied STL) and writes to stdout — no network, no secrets, no privileged access.
> `security_enforcement` is not explicitly set in config.json, but no security-relevant changes are in scope.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | N/A — no auth in this phase |
| V3 Session Management | no | N/A — no sessions |
| V4 Access Control | no | N/A — local CLI + browser WASM, no access control |
| V5 Input Validation | yes (CLI) | STL parsing already validates (stl_io); clap validates CLI args; `score_all_directions` guards against empty inputs (existing pattern) |
| V6 Cryptography | no | N/A — no crypto |

### Known Threat Patterns for Rust WASM + CLI

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malformed STL crashes parser | DoS | `parse_stl` returns `Result` + 5M triangle cap (already implemented in stl.rs:8) |
| NaN/Inf in metric computation | Tampering | All scoring functions guard with `is_finite()` and return 0.0 for non-finite (existing pattern in scoring.rs) |
| CLI path traversal | Tampering | CLI reads exact path from `std::env::args` — no globbing, no shell expansion. User runs locally. Low risk. |

## Sources

### Primary (HIGH confidence)
- `core/src/scoring.rs` — existing Rust metrics (score_candidate, footprint_area, max_cross_section, misalignment_score, min_z_height, shadowed_overhang_fraction, score_components) — VERIFIED by codebase read
- `core/src/stability.rs` — existing Rust stability (check_stability, convex_hull_2d, polygon_area, point_in_convex_polygon, min_edge_distance) — VERIFIED by codebase read
- `core/src/lib.rs` — existing WASM exports (prepare_data, score_orientation, refine_orientation, refine_orientation_batch) — VERIFIED by codebase read
- `web/src/compute.ts` — TS implementations being ported (rankByWeights, rankByConsensus, rankByTopsis, mergeCandidates, computeDefaultYaw, computeSlice) — VERIFIED by codebase read
- `.planning/phases/05-rust-consolidation/05-CONTEXT.md` — locked decisions — VERIFIED by direct user discussion
- `.opencode/skills/spike-findings-orient-stl/SKILL.md` — WASM rebuild rule (wasm-pack after every Rust edit) — VERIFIED

### Secondary (MEDIUM confidence)
- docs.rs/wasm-bindgen — confirmed wasm-bindgen 0.2.126 builds on x86_64-unknown-linux-gnu (dual-target compatibility) [CITED: docs.rs/wasm-bindgen]
- docs.rs/clap — clap 4.6.1 derive API tutorial (Parser, ValueEnum, Args) [CITED: docs.rs/clap/latest/clap/_derive/_tutorial/]
- rustwasm.github.io — wasm-bindgen closures example (Closure::new, js_sys::Function) [CITED: rustwasm.github.io/docs/wasm-bindgen/examples/closures.html]
- cargo search — serde_json 1.0.150 (17.7M wk), clap 4.6.1 (15.3M wk) [VERIFIED: cargo search + package-legitimacy check]

### Tertiary (LOW confidence)
- None — all findings verified via codebase read or authoritative docs

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — crates verified via cargo search + package-legitimacy check; existing code verified by codebase read
- Architecture: HIGH — feature-gating pattern confirmed via docs.rs build compatibility; WASM callback pattern confirmed via wasm-bindgen docs; CLI structure confirmed via clap docs
- Pitfalls: HIGH — all pitfalls derived from direct codebase analysis (existing code patterns, convention mismatches, build toolchain constraints)
- Ground-truth tests: HIGH — existing scoring/stability tests are already ground-truth; new test patterns follow the same structure

**Research date:** 2026-07-13
**Valid until:** 2026-08-12 (30 days — stable Rust ecosystem, no fast-moving dependencies)