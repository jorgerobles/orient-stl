# Roadmap: orient-stl

## Overview

A browser-based resin printing orientation tool. Rust WASM parses STL files, computes convex hulls, generates candidate orientations, scores them for overhang penalty and stability. A three.js viewport lets the user navigate the ranked list, adjust yaw with snap-to-geometry, and export oriented STLs — individually or as a ZIP bundle. The roadmap delivers a complete v1 MVP in two phases, then v2 algorithmic enhancements, then v3 UX polish.

## Phases

- [x] **Phase 1: Rust WASM Core Engine + Build Toolchain** — WASM `prepare_data()` parses STL, computes hull, generates + dedupes candidates; scoring/stability moved to JS workers (architecture drift — see 01-02-SUMMARY)
- [x] **Phase 2: Viewport + Yaw + Export (Complete UX Loop)** — three.js viewport with candidate navigation, yaw adjustment, single-file STL export
- [ ] **Phase 3: v2 Enhancements** — Height-weighted scoring, hull+sphere mode, hill-climbing refinement, multi-metric sorting, overhang heatmap (some items partially landed in Phase 1/2 drift)
- [ ] **Phase 4: v3 UX Polish** — Thumbnail strip, favorites persistence (IndexedDB), multi-STL ZIP export

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

### Phase 4: v3 UX Polish
**Goal**: Rich browsing experience with thumbnail strip, favorites persistence across sessions, and batch ZIP export
**Mode**: mvp
**Depends on**: Phase 3
**Requirements**: THUMB-01, THUMB-02, THUMB-03, FAV-01, FAV-02, FAV-03, EXPORT-03, EXPORT-04
**Success Criteria** (what must be TRUE):
  1. Thumbnail strip shows all top-N candidates as PNG images with score badges; clicking a thumbnail jumps to that candidate
  2. User can mark/unmark candidates as favorites with a visible toggle
  3. Favorites persist across page reloads (IndexedDB stores quaternion + thumbnail blob + metrics)
  4. User can export all favorite candidates as a single ZIP bundle; individual STLs named `model_orientNN_scoreX.stl`
**Plans**: 3 plans
**UI hint**: yes

Plans:
- [ ] 04-01: Thumbnail Strip (OffscreenCanvas rendering, Web Worker, score badge overlay)
- [ ] 04-02: Favorites (IndexedDB schema, CRUD operations, restore on reload)
- [ ] 04-03: ZIP Export (fflate bundle assembly, named STL files, download trigger)

## Progress

**Execution Order:** Phases execute in numeric order: 1 → 2 → 3 → 4

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Rust WASM Core Engine + Build Toolchain | 3/3 | ✅ Complete | 2026-07-11 |
| 2. Viewport + Yaw + Export (Complete UX Loop) | 3/3 | ✅ Complete | 2026-07-11 |
| 3. v2 Enhancements | 2/2 | ✅ Complete | - |
| 4. v3 UX Polish | 0/3 | Not started | - |

**Phase 2 detail (final):**
- 02-01 Viewport: ✅ complete (build plate, heatmap, centroid pivot, centering bug fixed)
- 02-02 Yaw: ✅ complete (linear slider + 45° snap; circular dial + geometry snap → Phase 3)
- 02-03 Export: ✅ complete
- Scoring: 4-metric consensus (overhang, footprint, cross-section, shadowed-overhang), yaw-optimized shadow (8-sample min), 100%→0% ranking
- Progress bar: segmented (per-worker) with paint-yield between sync phases
