# Roadmap: orient-stl

## Overview

A browser-based resin printing orientation tool. Rust WASM parses STL files, computes convex hulls, generates candidate orientations, scores them for overhang penalty and stability. A three.js viewport lets the user navigate the ranked list, adjust yaw with snap-to-geometry, and export oriented STLs — individually or as a ZIP bundle. The roadmap delivers a complete v1 MVP in two phases, then v2 algorithmic enhancements, then v3 UX polish.

## Phases

- [x] **Phase 1: Rust WASM Core Engine + Build Toolchain** — WASM `prepare_data()` parses STL, computes hull, generates + dedupes candidates; scoring/stability moved to JS workers (architecture drift — see 01-02-SUMMARY)
- [x] **Phase 2: Viewport + Yaw + Export (Complete UX Loop)** — three.js viewport with candidate navigation, yaw adjustment, single-file STL export
- [x] **Phase 3: v2 Enhancements** — Height-weighted scoring, hull+sphere mode, hill-climbing refinement, multi-metric sorting, overhang heatmap (some items partially landed in Phase 1/2 drift)
- [x] **Phase 3.5: Scoring Expansion & Refinement** — PrusaSlicer comparison surfaced 3 missing capabilities; all delivered (H5/H6 heuristics + profiles + seeded refine + TOPSIS + UI switcher)
- [x] **Phase 5: Consolidate All Calculations in Rust** — Move all metrics, rankings, selection, and yaw from TS to Rust. Single WASM call replaces worker-based computeSlice. Rust CLI binary for independent verification. TS becomes rendering-only. Ground-truth tests replace self-referential tests.
- [-] **Phase 4: v3 UX Polish** — Dropped (YAGNI — thumbnail strip, favorites, ZIP export not essential for core value proposition)
- [x] **Phase 6: Frontend Architecture Refactor** — Split god module, introduce state management, modularize Viewport, accessibility, CSS extraction, proper typing

## Phase Details

### Phase 1: Rust WASM Core Engine + Build Toolchain

**Goal**: WASM `compute_orientations()` accepts STL bytes + config, returns ranked `Candidate[]` with quaternion, penalties, stability flags through a typed JS boundary
**Mode**: mvp
**Depends on**: Nothing (first phase)
**Requirements**: STL-01, STL-02, STL-03, ORIENT-01, ORIENT-02, ORIENT-03, ORIENT-04, ORIENT-05, ORIENT-06, ORIENT-07, ORIENT-08, CONFIG-01, CONFIG-02
**Success Criteria** (what must be TRUE):

  1. User can select or drag-drop a binary STL file; raw bytes are passed to WASM
  2. WASM parses STL, precomputes per-triangle data, computes convex hull, generates deduplicated candidates from hull normals
  3. WASM scores each candidate for overhang penalty (area-weighted, configurable critical angle) and rejects unstable orientations
  4. Results returned as sorted `Candidate[]` with quaternion, penalties, stability flag — accessible as plain JS objects
  5. Config (critical angle, mode selector) changes affect scoring output

**Plans**: 3 plans

Plans:

- [x] 01-01-PLAN.md — WASM Build Toolchain: crate scaffolding (cdylib+rlib), Vite+TS project, wasm-pack build verification
- [x] 01-02-PLAN.md — Rust Compute Core: stl.rs, mesh.rs, hull.rs, candidates.rs, scoring.rs, stability.rs, lib.rs orchestration
- [x] 01-03-PLAN.md — JS Bridge & File Input: file picker + drag-drop, WASM init, compute_orientations() call, DOM result display

### Phase 2: Viewport + Yaw + Export (Complete UX Loop)

**Goal**: User sees candidates in a three.js viewport, navigates the ranked list, adjusts yaw with snap-to-geometry, and exports the oriented STL
**Mode**: mvp
**Depends on**: Phase 1
**Requirements**: STL-04, VIEW-01, VIEW-02, VIEW-03, VIEW-04, VIEW-05, YAW-01, YAW-02, YAW-03, YAW-04, YAW-05, EXPORT-01, EXPORT-02
**Success Criteria** (what must be TRUE):

   1. three.js viewport displays the model at the current candidate orientation with orbit/pan/zoom controls
   2. User can navigate next/prev candidate via buttons or keyboard; orientation changes instantly (quaternion copy); current rank and score are shown
   3. User adjusts yaw via linear slider with fixed 45° snap
   4. "Reset to auto" button restores the default bbox-minimizing yaw
   5. User exports the current orientation as a downloadable binary STL with transformed vertex positions

**Plans**: 3 plans
**UI hint**: yes

Plans:

- [x] 02-01: three.js Viewport (scene setup, OrbitControls, mesh display, candidate state management, navigation UI) ✅
- [x] 02-02: Yaw control (linear slider, 45° snap, reset) — circular dial + geometry snap deferred to Phase 3 overlay
- [x] 02-03: STL Export (binary STL writer in JS, quaternion application, Blob download trigger) ✅

### Phase 3: v2 Enhancements

**Goal**: Improved scoring accuracy through height-weighting, hull+sphere sampling, and interactive overlay with drag-to-rotate score feedback + hill-climb wizard
**Mode**: mvp
**Depends on**: Phase 2
**Requirements**: ORIENT-09, ORIENT-10, ORIENT-11, OVERLAY-01, OVERLAY-02, OVERLAY-03
**Success Criteria** (what must be TRUE):

   1. User can toggle hull+sphere mode; additional ~200 Fibonacci-sphere candidates appear in the ranked list
   2. Height-weighted scoring (k=0.5 multiplicative) improves ranking for models with tall overhangs
   3. User clicks a candidate → overlay mode with drag-to-rotate on the model + live score badge
   4. "Varita mágica" button in overlay runs hill-climb in Rust WASM from current orientation

**Plans**: 2 plans
**UI hint**: yes

Plans:

- [x] 03-01-PLAN.md — Rust WASM enhancements: Fibonacci sphere sampling, hull+sphere mode, hill-climb refine_orientation()
- [x] 03-02-PLAN.md — Interactive overlay: height-weighted scoring, hull+sphere toggle, drag-to-rotate with live score badge, Varita Mágica button

### Phase 4: v3 UX Polish — Dropped

**Goal**: Rich browsing experience with thumbnail strip, favorites persistence across sessions, and batch ZIP export
**Mode**: mvp
**Depends on**: Phase 3
**Requirements**: THUMB-01, THUMB-02, THUMB-03, FAV-01, FAV-02, FAV-03, EXPORT-03, EXPORT-04
**Success Criteria** (what must be TRUE): N/A — phase dropped before implementation

**Plans**: 3 plans (all cancelled)
**UI hint**: yes

**Decision:** Dropped as YAGNI — core value (orientation ranking) is fully delivered. Thumbnails, favorites, and ZIP export are nice-to-haves that don't affect print success. Single-file export via exportSTL covers the essential workflow.

Plans:

- [-] 04-01: Thumbnail Strip — Cancelled (YAGNI)
- [-] 04-02: Favorites — Cancelled (YAGNI)
- [-] 04-03: ZIP Export — Cancelled (YAGNI)

### Phase 3.5: Scoring Expansion & Refinement

**Goal**: Add heuristics discovered via PrusaSlicer codegraph comparison (H5 surface-quality, H6 print-height), make refine deterministic with a variance metric (H7), externalise weight profiles to JSON, and add a TOPSIS ranker with UI switcher
**Mode**: TDD (Red-Green-Refactor)
**Depends on**: Phase 3
**Requirements**: (none — research-driven expansion)
**Success Criteria** (what must be TRUE):

  1. `misalignmentScore` (TS) + `misalignment_score` (Rust) implement PrusaSlicer's "Best surface quality" objective ✅ (sub-plan 01)
  2. `min_z_height` (Rust) + existing `computeHeight` (TS) implement PrusaSlicer's "Lowest Z height" objective ✅ (sub-plan 01)
  3. `ScoreComponents` (Rust) + `Candidate`/`SliceResult` (TS) carry the new fields ✅ (sub-plan 01)
  4. `rankByWeights` and `rankByConsensus` both include all five heuristics ✅ (sub-plan 01)
  5. `WEIGHT_PRESETS` includes surface-only and height-only profiles ✅ (sub-plan 01)
  6. `core/src/rng.rs` provides seeded xorshift32 PRNG with determinism tests ✅
  7. `refine_orientation` accepts a `seed` param; `refine_orientation_batch` returns K×4 results ✅
  8. `computeSlice` calls batch refine (K=4), computes `refinedOverhang` (min) + `refineVariance` (stddev) ✅
  9. Weight profiles externalised to `web/src/profiles/*.json` with loader ✅
  10. `rankByTopsis(candidates, weights)` implements TOPSIS MCDA with tests ✅
  11. UI: profile dropdown + ranker dropdown in `main.ts` ✅
  12. All tests pass; WASM rebuilt; tsc clean ✅

Plans:

- [x] 03.5-01-PLAN.md — H5 + H6 heuristics, ScoreComponents/Candidate extension, ranker rewrites, Rust parity, WASM rebuild
- [x] 03.5-02-PLAN.md — Verify Rust spike (rng.rs + batch refine) + WASM rebuild; JSON profiles + loader + TOPSIS ranker (TDD); computeSlice batch refine + variance metric + UI dropdowns; final verification

### Phase 5: Consolidate All Calculations in Rust

**Goal**: One source of truth for all calculations. Rust computes every metric, every ranking, every selection, every yaw. TS is rendering-only. A Rust CLI binary verifies correctness independently of the browser.
**Mode**: TDD (Red-Green-Refactor)
**Depends on**: Phase 3.5
**Requirements**: (none — architecture consolidation driven by correctness concerns)
**Success Criteria** (what must be TRUE):

  1. Every scoring metric (overhang, footprint, cross-section, surface quality, height, shadowed overhang) has exactly ONE implementation — in Rust. No TS duplicate exists.
  2. Every ranking algorithm (weighted sum, consensus, TOPSIS) has exactly ONE implementation — in Rust. No TS duplicate exists.
  3. Candidate selection (angle-diversity merge) and yaw optimization (bbox-minimizing) run in Rust.
  4. A single WASM `score_all_directions` function replaces the worker-based `computeSlice` pipeline. Workers are eliminated or reduced to thin WASM dispatchers.
  5. A Rust CLI binary (`cargo run --bin cli -- file.stl`) runs the full pipeline (parse → hull → candidates → score → rank) and outputs JSON. Can verify correctness without a browser.
  6. All metric tests are ground-truth (hand-computed expected values from geometry/math), NOT self-referential (consistency-only tests that compare an implementation to itself).
  7. All TS metric/ranking test files are deleted. Only rendering-layer tests (quaternion, rotation, convention, centering) remain in TS.
  8. Browser UI produces identical results to CLI for the same STL + config.

Plans: 4 plans

Plans:

- [x] 05-01-PLAN.md — Rust ranking + selection + yaw modules with ground-truth tests; Cargo.toml dual-target feature scaffolding (TDD)
- [x] 05-02-PLAN.md — WASM `score_all_directions` / `rank_candidates` / `select_diverse` / `compute_norm_bounds` exports; shared `prepare_data_native` pipeline; drop self-referential tests; CLI binary `core/src/main.rs`
- [x] 05-03-PLAN.md — TS thin layer: strip `compute.ts` to types + `decimateForScore` + `WEIGHT_PRESETS`; delete `compute.test.ts`; simplify worker to single WASM dispatcher; update `main.ts`; single source-of-truth audit ✅
- [x] 05-04-PLAN.md — Cross-verification: CLI reference outputs for 12 (STL × ranker × profile) combinations + float-layout verification; single Rust source guarantees parity

## Progress

**Execution Order:** Phases execute in numeric order: 1 → 2 → 3 → 3.5 → 5 → 4 → 6

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Rust WASM Core Engine + Build Toolchain | 3/3 | ✅ Complete | 2026-07-11 |
| 2. Viewport + Yaw + Export (Complete UX Loop) | 3/3 | ✅ Complete | 2026-07-11 |
| 3. v2 Enhancements | 2/2 | ✅ Complete | - |
| 3.5 Scoring Expansion & Refinement | 2/2 | ✅ Complete | 2026-07-13 |
| 5. Consolidate All Calculations in Rust | 4/4 | ✅ Complete | 2026-07-13 |
| 4. v3 UX Polish | 0/3 | [-] Dropped (YAGNI) | 2026-07-14 |
| 6. Frontend Architecture Refactor | 4/4 | ✅ Complete | 2026-07-14 |

**Phase 2 detail (final):**

- 02-01 Viewport: ✅ complete (build plate, heatmap, centroid pivot, centering bug fixed)
- 02-02 Yaw: ✅ complete (linear slider + 45° snap; circular dial + geometry snap → Phase 3)
- 02-03 Export: ✅ complete
- Scoring: 4-metric consensus (overhang, footprint, cross-section, shadowed-overhang), yaw-optimized shadow (8-sample min), 100%→0% ranking
- Progress bar: segmented (per-worker) with paint-yield between sync phases

### Phase 6: Frontend Architecture Refactor

**Goal**: Transform the TS/HTML frontend from a monolithic imperative codebase into a modular, testable, accessible application. Split `main.ts` into controller + view classes, introduce explicit state management, extract `Viewport` sub-modules (GizmoController, DragHandler), move CSS to modules, add keyboard/ARIA support, and enforce typed message passing.
**Mode**: TDD (Red-Green-Refactor)
**Depends on:** Phase 5
**Success Criteria** (what must be TRUE):

  1. `main.ts` is split into `AppController`, `ScorePanel`, `ConfigPanel`, `CandidateList` (or equivalent classes), each ≤ 100 lines
  2. All mutable state lives in a single `AppState` object or store, not 14+ module-level `let` variables
  3. `Viewport` is split: `GizmoController` (ring creation, billboard, raycasting), `DragHandler` (pointer capture, angle math), `CameraRig` (position, reset) extracted
  4. `compute.ts` only contains `decimateForScore`; all types moved to `types.ts`
  5. CSS extracted to `styles/` directory with modules per component; no inline `<style>` in `index.html`
  6. Viewport rings are keyboard-accessible (arrow keys for rotation, tab navigation)
  7. Semantic HTML landmarks (`<header>`, `<main>`, `<aside>`) with ARIA attributes
  8. Worker messages typed via union type on the message envelope
  9. All magic numbers replaced with named constants in a config object
  10. `viewport.ts` and `main.ts` orchestration have unit test coverage
  11. Zero unused exports (e.g., `liftOntoPlate`)
  12. All empty catch blocks either removed or given explicit recovery

**Plans:** 4/4 plans executed

Plans:

- [x] 06-01-PLAN.md — Foundation: consolidate types.ts, create constants.ts, remove dead exports (liftOntoPlate/SliceResult/RefineFn), extract inline CSS to styles/
- [x] 06-02-PLAN.md — AppState store (EventTarget) + Viewport decomposition (GizmoController, DragHandler, CameraRig) with Pitfall 3 axis-mapping regression test
- [x] 06-03-PLAN.md — Split main.ts into AppController + view classes (ScorePanel, ConfigPanel, CandidateList, FileDrop) + typed worker messages (atomic)
- [x] 06-04-PLAN.md — Accessibility (keyboard rotation, ARIA, semantic HTML) + empty catch cleanup + final 12-criterion verification
