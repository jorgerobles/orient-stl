# Walking Skeleton — orient-stl

**Phase:** 1 — Rust WASM Core Engine + Build Toolchain
**Generated:** 2026-07-11

## Capability Proven End-to-End

> A user can load a binary STL file (via file picker or drag-drop) in a browser, which passes the raw bytes to a Rust WASM module that parses the mesh, computes a convex hull, generates ranked orientation candidates scored for overhang penalty and stability, and displays the candidate count and top results back in the DOM.

## Architectural Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Computation core | Rust → WASM (`wasm-bindgen 0.2`, `wasm32-unknown-unknown`) | Safe, fast, SIMD through LLVM. Coarse-grained single-call boundary avoids marshalling overhead. |
| Build toolchain | `wasm-pack 0.15+` targeting bundler | Handles wasm-bindgen codegen, wasm-opt, and npm package generation. Standard for Rust WASM projects. |
| JS framework | None (vanilla TypeScript + Vite 8.1) | No framework overhead. Single-page tool with one viewport and one compute call. Vite provides fast dev server and WASM module support. |
| WASM integration | `vite-plugin-wasm 3.6` + `vite-plugin-top-level-await 1.5` | Enables natural `import init, { compute_orientations } from 'orient-core'` syntax. Standard Vite WASM setup. |
| STL parsing | `stl-io 0.11` in Rust (binary only) | Zero-dependency crate, verified compiles to wasm32-unknown-unknown. Pass raw file bytes from JS as `&[u8]`. |
| Convex hull | Vendored incremental quickhull (~300 lines, f32) | Avoids transitive dependency issues of external crates (ndarray, BLAS, rayon) on wasm target. Full control over precision. |
| Data serialization | `serde-wasm-bindgen 0.6` (plain JS objects) | Produces plain JS objects with no WASM memory handles. No manual `.free()` needed. JS owns the result data independently. |
| WASM execution | Main thread (no Web Worker, no threading) | Estimated compute time 30-80ms for typical resin models (100K triangles, 100 candidates). Worker complexity not justified. |
| Config object | Single `OrientConfig` struct deserialized via serde-wasm-bindgen | Backward-compatible: adding config fields doesn't break existing callers. Flat struct with `#[serde(default)]`. |
| Up axis | +Z is up | Standard 3D printing convention. No configurable up-axis in v1. |
| Yaw computation | Rust-side rotating calipers on hull vertices | Reuses hull data already in WASM memory. Avoids JS-side geometry copy. |
| STL parsing location | Inside WASM (`stl.rs`) | JS passes raw `Uint8Array` bytes. WASM parses once, extracts positions, precomputes mesh data. No JS-side STL knowledge needed. |
| Result display | DOM-based list (no three.js in Phase 1) | three.js viewport deferred to Phase 2. Phase 1 displays candidate metrics as text for pipeline verification. |

## Stack Touched in Phase 1

- [x] Project scaffold (Rust crate + Vite + TypeScript + wasm-pack build)
- [x] WASM module — at least one `#[wasm_bindgen]` function called from JS
- [x] STL parsing — binary STL read from `&[u8]` via stl-io
- [x] Geometry computation — mesh precomputation, convex hull, candidate generation, scoring, stability
- [x] UI — file picker, drag-drop, config panel, result display
- [x] Deployment — `npm run dev` runs full stack locally
- [ ] Viewport — deferred to Phase 2 (three.js)

## Out of Scope (Deferred to Later Slices)

- three.js viewport and candidate navigation — Phase 2
- Yaw adjustment dial — Phase 2
- STL export (single or ZIP) — Phase 2
- Height-weighted scoring — Phase 3
- `hull_plus_sphere` candidate mode — Phase 3
- S² hill-climbing refinement — Phase 3
- Overhang heatmap visualization — Phase 3
- Thumbnail strip (OffscreenCanvas) — Phase 4
- Favorites / IndexedDB persistence — Phase 4
- Multi-STL ZIP export — Phase 4
- ASCII STL support — deferred indefinitely (binary covers all common cases)
- Web Worker / threading — deferred (measure first — only if >200ms compute time)

## Subsequent Slice Plan

- **Phase 2**: three.js viewport with candidate navigation (next/prev), circular yaw dial with snap-to-geometry, single STL export
- **Phase 3**: v2 algorithmic enhancements — height-weighted scoring, hull+sphere mode, hill-climbing refinement, multi-metric sorting, overhang heatmap
- **Phase 4**: v3 UX polish — thumbnail strip, favorites persistence (IndexedDB), multi-STL ZIP export (fflate)

## Repository Structure

```
orient-stl/
├── core/                          # Rust crate (wasm-bindgen)
│   ├── Cargo.toml                 # cdylib + rlib, wasm-bindgen, stl-io, serde
│   └── src/
│       ├── lib.rs                 # #[wasm_bindgen] — compute_orientations() only
│       ├── stl.rs                 # Binary STL parser (stl-io wrapper)
│       ├── mesh.rs                # Per-triangle normal/area/centroid precompute
│       ├── hull.rs                # Vendored incremental quickhull (~300 lines, f32)
│       ├── candidates.rs          # Hull normal → candidate directions + dedupe + yaw
│       ├── scoring.rs             # Area-weighted overhang penalty (S² space)
│       └── stability.rs           # Footprint + CoM + point-in-polygon stability check
├── web/                           # Vite + TypeScript frontend
│   ├── index.html                 # App shell with file input, drop zone, results, config
│   ├── package.json               # Vite, vite-plugin-wasm, TypeScript
│   ├── vite.config.ts             # wasm + top-level-await plugins
│   ├── tsconfig.json              # ESNext, bundler module resolution
│   ├── pkg/                       # wasm-pack output (gitignored)
│   └── src/
│       ├── main.ts                # WASM init, file picker, drag-drop, result display
│       ├── loadSTL.ts             # File → Uint8Array → WASM bridge
│       └── types.ts               # Candidate, OrientConfig interfaces
├── .planning/                     # GSD planning directory
└── orient-spec.md                 # Project specification
```

## Dev Commands

```bash
# One-time setup
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
cd web && npm install

# Build WASM module
wasm-pack build core --target bundler --out-dir ../web/pkg

# Full rebuild (WASM + Vite)
cd web && npm run rebuild

# Development
cd web && npm run dev        # starts Vite on localhost:5173
```
