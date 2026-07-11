# Project Research Summary

**Project:** orient-stl
**Domain:** Resin 3D printing auto-orientation tool (Rust WASM + three.js browser app)
**Researched:** 2026-07-11
**Confidence:** HIGH

## Executive Summary

orient-stl is a specialized browser-based tool for computing optimal 3D print orientations for resin STL models. Experts build this class of product as a **two-domain system**: a Rust WASM computation core handles all geometry math (parsing, hull, candidate generation, scoring, stability), while a JavaScript host manages the three.js viewport, UI, and browser persistence. The critical architectural insight is that the WASM↔JS boundary must be **coarse-grained** — a single `compute_orientations()` call crosses the boundary once with input and once with output, keeping all intermediate computation (50–300 modules of Rust code) inside WASM memory.

The recommended approach from all four research outputs converges strongly: **Rust WASM for computation (stl-io, vendored quickhull, serde-wasm-bindgen) + three.js + Vite for UI + fflate for export**, built in phases that follow the data pipeline. The core differentiator — a ranked candidate list with multi-metric scoring — is achievable entirely in ~80ms of WASM compute for typical resin models (100K triangles, 100 candidates), which means v1 can run on the main thread without Web Workers or threading complexity.

The key risks are: (1) leaking WASM memory by using `#[wasm_bindgen]` struct handles instead of `serde-wasm-bindgen` serialization, (2) premature threading before measuring actual compute time, and (3) forgetting `await init()` before calling WASM functions. All three are well-documented and straightforward to prevent with discipline. The most significant unresolved tension is whether to use three.js STLLoader for viewport display or have WASM return geometry data — a decision that affects the v2 planning boundary.

## Key Findings

### Recommended Stack

The stack is well-established with HIGH confidence across all sources. Rust stl-io compiles cleanly to `wasm32-unknown-unknown`, wasm-bindgen handles the JS bridge, and three.js is the de facto WebGL viewport.

**Core technologies:**
- **Rust (stable 1.96+)**: Computation core — safe, fast, SIMD through LLVM, wasm32 target
- **wasm-bindgen (0.2.x)**: JS↔WASM bridge — generates `.d.ts`, handles zero-copy slices, mature
- **wasm-pack (0.13+)**: Build orchestrator — compiles, runs wasm-bindgen + wasm-opt, generates npm package
- **stl-io (0.11.x)**: STL binary parsing — zero-dependency, verified compiles to wasm32-unknown-unknown
- **serde-wasm-bindgen (0.6.x)**: Structured data serialization — produces plain JS objects (no manual `.free()`)
- **three.js (latest stable)**: 3D viewport — OrbitControls, quaternion support, OffscreenCanvas
- **Vite 5+**: Build tool — fast HMR, native WASM module support
- **fflate (latest)**: ZIP compression — zero-dependency, browser-compatible, faster than JSZip
- **vendored quickhull (~300 lines)**: Convex hull — avoids transitive dependency issues of external crates; fully controlled

**Key decisions confirmed:**
- Serialize results with `serde-wasm-bindgen`, NOT `#[wasm_bindgen]` struct handles (AGREED across all 4 research files)
- Single `compute_orientations()` function, NOT fine-grained WASM API (AGREED)
- Main-thread WASM in v1, NOT threading (AGREED — benchmark estimate: 30-80ms per mesh)
- Binary-only STL in v1, defer ASCII (AGREED — spec confirms this)

### Expected Features

**Must have (table stakes):**
- STL file loading (binary) — drag-drop or file picker, feed raw bytes to WASM
- 3D viewport with orbit controls — three.js scene at current candidate orientation
- Candidate navigation — next/prev through ranked list with instant quaternion flip
- STL export (single file) — apply candidate quaternion, download as binary STL
- Yaw adjustment — circular dial for fine-tuning around vertical axis

**Should have (competitive differentiators):**
- **Ranked candidate list** — the core differentiator. Browse ~50 candidates sorted by score.
- **Multi-metric ranking** — overhang, height, stability scores shown separately
- **Overhang penalty visualization** — heatmap on model faces for the selected candidate
- **Stability check** — reject/reduce-rank orientations that would fall over
- **Yaw snap to geometry** — snap to bounding-box minima via rotating calipers
- **Multi-file ZIP export** — batch export several candidate orientations at once

**Defer (v2+):**
- Thumbnail strip (v3) — OffscreenCanvas + Web Worker
- Favorites / IndexedDB persistence (v3)
- Multi-metric sort UI (v2) — v1 ships with composite score only
- Height-weighted scoring (v2) — v1 uses area-weighted only
- S² hill-climbing refinement (v2) — hull candidates are good enough for v1
- `hull_plus_sphere` mode (v2) — hull mode alone covers most models

**Anti-features (will NOT build):**
- Full slicer / gcode generation — output oriented STLs, let slicers handle the rest
- Support generation — minimize supports via orientation, not generate them
- Mesh repair / decimation — assume clean manifold STLs
- Cloud / network features — all computation in-browser, no data leaves the machine
- Real-time orientation drag — pre-compute ranked list, navigate with instant quaternion flips
- Multi-model packing — separate problem; orient one model at a time

### Architecture Approach

The system splits into two isolated domains with a narrow, typed boundary. Rust WASM owns all computation (STL parse → mesh precomputation → hull → candidates → scoring → stability → refinement). The JS host owns all UI (file I/O, three.js viewport, thumbnail worker, export, persistence). Data flows in a unidirectional pipeline: **File → WASM parse → WASM compute → JS display → User interaction → Export**.

**Major components (Rust side):**
1. **`stl.rs`**: STL binary parser (stl-io wrapper) — no JS dependencies, zero-copy from `&[u8]`
2. **`mesh.rs`**: Per-triangle normal, area, centroid precomputation — runs once per mesh
3. **`hull.rs`**: Vendored quickhull (~300 lines) — convex hull for candidate direction generation
4. **`candidates.rs`**: Candidate generation from hull normals + deduplication + yaw computation
5. **`scoring.rs`**: Overhang penalty scoring (S² space) — O(n_triangles × n_candidates), the performance-critical loop
6. **`stability.rs`**: Footprint + center-of-mass + point-in-polygon — binary reject in v1
7. **`refine.rs`**: S² hill-climbing refinement on top-K candidates — v2 feature
8. **`lib.rs`**: Orchestrates all modules. **Only** file that exports `#[wasm_bindgen]` items.

**Major components (JS side):**
1. **`main.ts`**: App entry point, WASM init, file input handler, candidate state management
2. **`loadSTL.ts`**: FileReader → ArrayBuffer → WASM bridge
3. **`viewport.ts`**: three.js scene with OrbitControls, next/prev navigation, candidate display
4. **`yaw-dial.ts`**: Circular dial for yaw adjustment with snap-to-geometry
5. **`exportSTL.ts`**: Single STL + fflate ZIP export
6. **`thumbnails.ts`** (v3): OffscreenCanvas + Web Worker for thumbnail strip
7. **`favorites.ts`** (v3): IndexedDB persistence for favorites

**Key patterns:**
- Coarse-grained boundary: single `compute_orientations()` call
- Config object pattern: `serde-wasm-bindgen` deserialization of `OrientConfig`
- Module isolation in Rust: `pub(crate)` visibility, only `lib.rs` exports WASM bindings
- Borrow and copy at boundary: WASM borrows input via `&[f32]`, returns serialized plain objects
- main-thread WASM in v1; Worker-based thumbnails in v3; threading NOT recommended

### Critical Pitfalls

1. **Leaking WASM memory via `#[wasm_bindgen]` struct handles** — Using `#[wasm_bindgen]` structs instead of `serde-wasm-bindgen` serialization creates JS class handles that must be manually `.free()`'d. For 100 candidates, 100 handles. **Prevention:** Always use serde-wasm-bindgen for structured output. WASM memory is released when the function returns.

2. **Premature WASM threading** — Setting up `wasm-bindgen-rayon`, SharedArrayBuffer, COOP/COEP headers, nightly Rust before measuring is 10× build complexity for no perceptible benefit (compute is 30-80ms on main thread). **Prevention:** Benchmark first. Only reach for Workers if >300ms on target hardware.

3. **Calling WASM before `init()` resolves** — The `init()` call is async (fetches + instantiates WASM binary). Forgetting `await init()` produces opaque `TypeError: compute_orientations is not a function`. **Prevention:** Store a `wasmReady` flag. `await init()` at app startup before revealing the file input.

4. **Fine-grained WASM boundary** — Exporting 10+ small WASM functions and calling them sequentially from JS makes boundary overhead dominate performance. **Prevention:** Single `compute_orientations()` function. WASM orchestrates internally.

5. **Dual STL parsing (three.js STLLoader + WASM)** — Parsing STL in three.js for display AND parsing again in WASM for computation wastes time and memory. **Prevention:** Resolve during v2 planning whether WASM returns geometry data or JS does a minimal parse for display.

**Phase-specific warnings:**
- Phase 1 (WASM core): missing `cdylib + rlib` crate types, forgetting `await init()`, no deduplication
- Phase 2 (Viewport): decision needed on viewport geometry source
- Phase 3 (Yaw + Export): rotating calipers validation needed
- Stability check: numerical edge cases in point-in-polygon (epsilon tolerance, winding number)

## Implications for Roadmap

### Suggested Phase Structure

#### Phase 1: Rust WASM Core Engine
**Rationale:** Everything depends on the computation engine. The WASM module defines the data structures, the boundary API, and the entire compute pipeline. Build this first because it has no JS runtime dependencies and can be tested independently with `cargo test`.

**Delivers:** A complete `compute_orientations()` WASM function that accepts file bytes + config and returns ranked, deduplicated, stability-checked candidates. Delivers build toolchain (wasm-pack + Vite + vite-plugin-wasm).

**Addresses:** STL loading, candidate generation, scoring, stability check (FEATURES.md table stakes + differentiators)

**Rust modules built:** `stl.rs`, `mesh.rs`, `hull.rs`, `candidates.rs`, `scoring.rs`, `stability.rs`, `lib.rs`

**Avoids:** Pitfall 4 (fine-grained boundary by design), Pitfall 7 (double crate type in Cargo.toml), Pitfall 6 (panic hook), Pitfall 12 (deduplication required)

**Research flag:** Build toolchain and module structure are well-documented standard patterns. Skip research-phase for Phase 1.

#### Phase 2: three.js Viewport + Navigation
**Rationale:** Once the WASM engine returns candidates, the user needs to see and navigate them. The viewport is purely a display concern — no computation. three.js with OrbitControls is a standard pattern.

**Delivers:** 3D scene showing the model at the current candidate orientation, next/prev navigation to cycle through the ranked list, quaternion interpolation between candidates.

**Addresses:** 3D viewport, candidate navigation (FEATURES.md table stakes)

**Unresolved decision:** How does the viewport get geometry? Three options to resolve during Phase 2 planning:
1. WASM returns vertex data alongside candidates (adds to output size)
2. JS does a minimal STL binary parse for display only (~50 lines)
3. Use three.js STLLoader separately (two parses, but simplest)

**Avoids:** Pitfall 5 (dual STL parsing — resolve during planning), Pitfall 3 (WASM init — already handled in Phase 1)

**Research flag:** Needs deeper research during planning to resolve the viewport geometry source decision. This is the single unresolved architectural question.

#### Phase 3: Yaw Adjustment + STL Export
**Rationale:** Phases 1+2 give the user a ranked list they can view. Phase 3 completes the core UX loop: pick a candidate, fine-tune yaw, export the oriented STL. Export is O(n) in triangles and avoids WASM boundary calls.

**Delivers:** Circular yaw dial with snap-to-geometry (rotating calipers), single file STL export (quaternion applied to vertices).

**Addresses:** Yaw adjustment, STL export (FEATURES.md table stakes)

**Resolves:** Yaw snap reuses rotating calipers already computed in Phase 1 hull computation.

**Avoids:** Pitfall anti-pattern 5 (re-parsing STL on export — read from cached geometry directly)

**Research flag:** Export patterns are standard. Minimal research needed — skip.

#### Phase 4: v2 Enhancements — Refinement, Multi-Metric, Height-Weighted
**Rationale:** The core workflow is complete after Phase 3. Phase 4 adds algorithmic improvements that make results better but don't change the UX architecture.

**Delivers:** `refine.rs` (S² hill-climbing on top-K candidates), multi-metric sorting UI (sort by overhang/height/stability), height-weighted scoring, `hull_plus_sphere` mode for non-standard models.

**Addresses:** Refinement, multi-metric ranking (FEATURES.md differentiators)

**Research flag:** Hill-climbing on S² has well-documented math. Low research risk. Skip.

#### Phase 5: v3 UX Polish — Thumbnails, Favorites, ZIP Export
**Rationale:** These features enhance the UX but are independent of the compute pipeline. Thumbnails use OffscreenCanvas + Web Worker. Favorites use IndexedDB. ZIP export reuses single-file export logic.

**Delivers:** Thumbnail strip (OffscreenCanvas Worker), favorites persistence (IndexedDB with PNG thumbnails), multi-file ZIP download (fflate).

**Addresses:** Thumbnails, favorites, ZIP export (FEATURES.md deferred features)

**Avoids:** Pitfall on OffscreenCanvas compatibility (provide main-thread fallback)

**Research flag:** OffscreenCanvas + Web Worker has documented three.js patterns. Low risk. Skip research-phase.

### Phase Ordering Rationale

- **Phase 1 first** because all other phases depend on the WASM compute output and the build toolchain. Without `compute_orientations()`, there's nothing to display, navigate, or export.
- **Phase 2 before Phase 3** because yaw adjustment and export both require the user to see and select a candidate first.
- **Phase 4 before Phase 5** because algorithmic improvements (refinement, multi-metric) improve the core product, while thumbnails/favorites/ZIP are UX polish. But Phase 4 and Phase 5 could run in parallel if needed.
- Stability check included in Phase 1 (not deferred) because it's critical for resin printing success per the feature analysis.
- Refinement, multi-metric, height-weighted scoring are deferred to Phase 4 because hull-mode candidates are good enough for v1.
- No phase requires threading or SharedArrayBuffer complexity.

### Research Flags

Phases needing deeper research during planning:
- **Phase 2 (Viewport):** The viewport geometry source decision needs resolution. Three options exist; the choice affects the WASM output contract. Needs a spike or decision-making session during Phase 2 planning.

Phases with standard patterns (skip research-phase):
- **Phase 1 (WASM Core):** Well-established patterns. wasm-bindgen + stl-io + quickhull is standard Rust WASM.
- **Phase 3 (Yaw + Export):** Straightforward feature additions to existing architecture.
- **Phase 4 (v2 Enhancements):** Refinement is standard S² gradient descent. Multi-metric is a UI sort widget.
- **Phase 5 (v3 UX Polish):** OffscreenCanvas + IndexedDB patterns are documented in three.js and MDN.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All technologies verified against official docs (wasm-bindgen, stl-io, three.js, Vite). One conflict: FEATURES.md suggests JS-side STL parsing for viewport, ARCHITECTURE recommends WASM-only. |
| Features | HIGH | Sourced from project spec + competitive analysis (PrusaSlicer, Lychee, AutoOrientation). MVP prioritization is clear and internally consistent. |
| Architecture | HIGH | The two-domain split, coarse-grained boundary, and module structure are well-documented patterns. All anti-patterns are supported by concrete wasm-bindgen guidance. |
| Pitfalls | HIGH | Every pitfall is sourced from official docs or common wasm-bindgen community knowledge. |

**Overall confidence:** HIGH

### Gaps to Address

- **Viewport geometry source (Phase 2):** The research files don't fully agree on whether the viewport should get geometry from WASM or from a separate JS parse. This must be decided during Phase 2 planning. Recommended resolution: have WASM return vertex positions alongside candidates (small additional output, avoids double-parse).
- **STL ASCII support:** All research recommends binary-only in v1, with ASCII deferred. No real-world validation of whether users commonly encounter ASCII STL files. Flag for user testing during Phase 1.
- **Performance benchmarks:** The 30-80ms WASM compute estimate is informed but not yet measured. Actual benchmark on target hardware should happen during Phase 1 to validate the main-thread decision.
- **Yaw computation location:** ARCHITECTURE suggests yaw computation in Rust (reusing rotating calipers). FEATURES doesn't specify. Decision needed in Phase 3: yaw snap in WASM (reuses hull data) or JS (simpler but duplicates computation). WASM recommended.

## Sources

### Primary (HIGH confidence)
- wasm-bindgen Guide — JS↔WASM boundary patterns, common pitfalls. https://rustwasm.github.io/docs/wasm-bindgen/
- stl-io crate — STL binary parsing for wasm32. https://crates.io/crates/stl-io
- serde-wasm-bindgen — Structured data serialization. https://crates.io/crates/serde-wasm-bindgen
- Vite WASM plugin — WASM module import in Vite. https://github.com/nicolo-ribaudo/vite-plugin-wasm
- Three.js OffscreenCanvas manual — Worker rendering patterns. https://threejs.org/manual/en/offscreencanvas.html
- wasm-pack — Build toolchain for Rust WASM. https://rustwasm.github.io/wasm-pack/

### Secondary (MEDIUM confidence)
- Rust for TS/JS Developers (2026) — WASM performance analysis. https://rs4ts.dev/19-wasm/
- wasm-bindgen Raytrace example — Threading caveats. https://rustwasm.github.io/docs/wasm-bindgen/examples/raytrace.html
- fflate documentation — ZIP compression for browser. https://github.com/101arrowz/fflate
- Community knowledge: Quickhull edge cases, point-in-polygon numerical stability

### Tertiary (LOW confidence)
- Competitor analysis (PrusaSlicer, Lychee) — Feature landscape inferred from product observation, not official documentation
- Resin printing community — Overhang angles and stability requirements from forum knowledge, not primary sources

---
*Research completed: 2026-07-11*
*Ready for roadmap: yes*
