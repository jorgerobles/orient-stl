# Architecture Patterns

**Project:** orient-stl
**Researched:** 2026-07-11
**Domain:** Rust WASM + three.js browser application for STL orientation optimization
**Overall confidence:** HIGH

## Recommended Architecture

The system splits into two isolated domains connected by a narrow, typed boundary: **Rust WASM core** (computation) and **JS host** (UI, rendering, persistence). Data flows in a unidirectional pipeline: File → WASM parse → WASM compute → JS display → User interaction → Export.

```
┌─────────────────────────────────────────────────────┐
│                    JS Host (Vite)                     │
│                                                       │
│  FileReader ──► loadSTL.ts ──► ArrayBuffer ───┐      │
│                                               │      │
│                 ┌─────────────────────────────┘      │
│                 ▼                                    │
│         WASM init() ──► compute_orientations()       │
│                 │                                    │
│                 ▼                                    │
│         Candidate[] returned to JS                   │
│                 │                                    │
│         ┌───────┼───────────┬─────────────┐          │
│         ▼       ▼           ▼             ▼          │
│    viewport.ts  thumbnails.ts  favorites.ts  main.ts │
│    (three.js)   (offscreen    (IndexedDB)   (UI)     │
│                  canvas)                              │
│         │       │           │             │          │
│         └───────┴───────────┴─────────────┘          │
│                 │                                    │
│         ┌───────┴───────────┐                        │
│         ▼                   ▼                        │
│    yaw adjust          exportSTL.ts                  │
│    (dial UI)           (fflate ZIP)                  │
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│              Rust WASM Core (wasm32-unknown-unknown)  │
│                                                       │
│  lib.rs (API surface)                                 │
│   │                                                    │
│   ├── stl.rs        (STL binary parse)                │
│   ├── mesh.rs       (normal/area/centroid precompute) │
│   ├── hull.rs       (convex hull, quickhull)          │
│   ├── candidates.rs (hull | hull_plus_sphere, dedupe) │
│   ├── scoring.rs    (overhang penalty, S² space)      │
│   ├── stability.rs  (footprint, CoM, point-in-poly)   │
│   └── refine.rs     (S² hill-climbing)                │
│                                                       │
│  All modules internal. Only lib.rs exports #[wasm_bindgen] items.
└─────────────────────────────────────────────────────┘
```

## Component Boundaries

### The WASM ↔ JS Boundary

This is the single most important architectural decision. The boundary must be **narrow and coarse-grained** — each crossing has marshalling overhead.

**Recommended: A single `#[wasm_bindgen]` function `compute_orientations()` that accepts a flat `&[f32]` buffer and a serialized config object, and returns serialized candidates.** This means one crossing in, one crossing out. All intermediate computation (parsing, precomputation, hull, candidate generation, scoring, dedupe, refinement) stays inside WASM.

**Boundary API contract:**

```rust
// lib.rs — the ONLY file that exports #[wasm_bindgen] items

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn compute_orientations(positions: &[f32], config: &JsValue) -> JsValue {
    // 1. Parse config from JsValue
    // 2. Call internal modules (mesh, hull, candidates, scoring, stability, refine)
    // 3. Serialize result back to JsValue
    // One crossing in, one crossing out
}
```

```typescript
// JS side — single import, single call
import init, { compute_orientations } from './pkg/orient_core.js';

await init();
const result: Candidate[] = compute_orientations(positions, config);
```

### What Lives Where

| Capability | Owner | Why |
|------------|-------|-----|
| STL binary parsing | Rust (`stl.rs`) | WASM has direct memory access to raw bytes; avoids JS-side copy. stl-io crate compiles cleanly to wasm32. |
| Mesh precomputation (normals, areas, centroids) | Rust (`mesh.rs`) | Pure math, no DOM access needed. Runs once per mesh. |
| Convex hull | Rust (`hull.rs`) | O(n log n) geometry algorithm. Vendored quickhull (~300 lines). |
| Candidate generation | Rust (`candidates.rs`) | Operates on hull normals + sphere samples. Pure computation. |
| Overhang scoring | Rust (`scoring.rs`) | O(n_triangles × n_candidates). Must be fast. The entire point of using WASM. |
| Stability check | Rust (`stability.rs`) | ~40 lines of geometry predicates. |
| Refinement (hill-climbing) | Rust (`refine.rs`) | Iterative S² search. Pure computation. |
| Yaw computation | Rust (in `candidates.rs` or separate) | Reuses rotating calipers from hull. |
| File I/O | JS (`loadSTL.ts`) | Browser File API is JS-only. FileReader → ArrayBuffer → WASM. |
| three.js viewport | JS (`viewport.ts`) | three.js is a JS library. No Rust bindings. |
| Offscreen thumbnail rendering | JS (`thumbnails.ts`) | three.js in Web Worker with OffscreenCanvas. |
| IndexedDB persistence | JS (`favorites.ts`) | Browser storage API, JS-only. |
| ZIP export | JS (`exportSTL.ts`) | fflate is a JS library. |
| Yaw dial UI | JS (main.ts / viewport.ts) | DOM event handling for the circular dial interaction. |
| UI state management | JS (main.ts) | Navigation, selection, button handlers. |

### Data Structures at the Boundary

**Input to WASM:**
- `positions: &[f32]` — flat array of vertex positions (9 floats per triangle, 3 vertices × 3 coords). WASM gets zero-copy access via wasm-bindgen's slice marshalling.
- `config: JsValue` — serialized `OrientConfig` object. Use `serde-wasm-bindgen` for clean deserialization on the Rust side.

**Output from WASM:**
- Return type: `JsValue` — serialized `Vec<Candidate>`. Use `serde-wasm-bindgen` to serialize from a Rust `Vec<Candidate>` to a JS array of plain objects.

**Why `JsValue` over `#[wasm_bindgen] struct`:**
- `#[wasm_bindgen]` structs become JS classes with handles to WASM memory that need manual `.free()`. For a result set you display and navigate, you'd hold handles to 50-100 candidates. Each access (e.g., reading `candidate.overhangPenalty`) crosses the boundary. This is fine for a handful of accesses but couples JS lifetime to WASM memory.
- `serde-wasm-bindgen` serialization produces plain JS objects with no lifetime coupling. WASM memory can be released after the call returns. JS can freely manipulate the results, sort them, cache them, pass them to Workers.
- **Confidence: HIGH** — serde-wasm-bindgen is the standard pattern for returning structured data. The marshalling cost is paid once and is negligible for 100 candidates.

**Rejected alternative: Typed array with manual offsets**
- Manually writing `f32` values into a `Vec<f32>` and returning a typed array would avoid the serde overhead, but the code complexity is not worth it. serde-wasm-bindgen handles ~100 candidates in <1ms.

### Incremental Dev API (not for production)

During development, expose individual module functions for testing in the browser console:

```rust
#[wasm_bindgen]
pub fn dev_compute_normals(positions: &[f32]) -> JsValue { /* ... */ }

#[wasm_bindgen]
pub fn dev_compute_hull(positions: &[f32]) -> JsValue { /* ... */ }
```

Gate behind `#[cfg(debug_assertions)]` or a Cargo feature `dev-api` so they don't bloat the release binary.

## Data Flow: End-to-End Pipeline

### Phase 1: File Load

```
User selects file
    │
    ▼
FileReader.readAsArrayBuffer(file)
    │
    ▼
new Uint8Array(arrayBuffer)           ← JS owns the bytes
    │
    ▼
WASM compute_orientations(bytes, config)
    │
    ├── stl.rs: parse STL header + triangles → Vec<Triangle>
    ├── mesh.rs: compute normal, area, centroid per triangle
    ├── hull.rs: compute convex hull from vertices (for hull mode)
    ├── candidates.rs: generate candidate directions (hull normals | sphere)
    ├── scoring.rs: score each candidate (overhang penalty + height)
    ├── stability.rs: check stability for each candidate
    ├── refine.rs: S² hill-climb on top-K candidates
    │
    ▼
Returns JsValue (Candidate[])
    │
    ▼
JS receives Candidate[] (plain objects with quaternion, scores, etc.)
```

**Critical optimization:** The flat `positions: &[f32]` buffer sent to WASM in step 1 is the **same** buffer used for STL parsing. WASM reuses the raw triangle data in-place without copying. Mesh precomputation (normals, areas, centroids) computes derived data once and stores it in WASM memory for all subsequent operations.

### Phase 2: Display

```
JS receives Candidate[] sorted by compositeScore
    │
    ▼
viewport.ts: mesh.quaternion.copy(candidates[0].quaternion)
    │
    ▼
User presses Next → mesh.quaternion.copy(candidates[i++].quaternion)
    │
    ▼
Mesh orientation changes instantly — no geometry reload, no WASM call
```

**Key insight:** The mesh geometry loaded by three.js's STLLoader (or a minimal JS parser) stays fixed. Only `mesh.quaternion` changes between candidates. This means orientation switching is O(1) and completely smooth.

### Phase 3: Thumbnails (v3+)

```
Candidate[] in JS
    │
    ▼
For top N candidates:
    ├── Spawn Web Worker with OffscreenCanvas
    ├── Worker runs three.js scene (model at candidate orientation)
    ├── Render to OffscreenCanvas → transfer back as ImageBitmap
    └── Display in thumbnail strip
```

### Phase 4: Export

```
User marks favorites
    │
    ▼
For each favorite:
    ├── Read original mesh geometry
    ├── Apply candidate quaternion
    ├── STL-encode rotated mesh
    └── Collect binary blobs
    │
    ▼
If 1 favorite: download single STL
If 2+ favorites: fflate ZIP → download ZIP
```

## Build Order Dependencies

### Module Dependency Graph

```
stl.rs (no deps)
   │
   ▼
mesh.rs (needs: raw triangles from stl.rs → normals, areas, centroids)
   │
   ▼
hull.rs (needs: vertices from mesh.rs → convex hull)
   │
   ▼
candidates.rs (needs: hull normals from hull.rs, optional sphere sampling)
   │
   ▼
scoring.rs (needs: mesh data from mesh.rs, candidate directions from candidates.rs)
   │
   ▼
stability.rs (needs: hull from hull.rs, mesh from mesh.rs, candidate directions)
   │
   ▼
refine.rs (needs: scoring, stability, candidate directions)
   │
   ▼
lib.rs (orchestrates all of the above)
```

### Phase Build Order (recommended implementation sequence)

| Step | Module | Depends On | Why First |
|------|--------|-----------|-----------|
| 1 | `stl.rs` | Nothing | Foundation. Need to parse files to have data. |
| 2 | `mesh.rs` | `stl.rs` | Precomputes per-triangle data needed by everything else. |
| 3 | `lib.rs` skeleton | `stl.rs`, `mesh.rs` | WASM binding setup, build toolchain verification. Wire up a trivial `compute_orientations` that just parses and returns. Validate JS↔WASM boundary works. |
| 4 | `scoring.rs` | `mesh.rs` | Core algorithm. Score is the primary output. Implement with hardcoded test candidates first. |
| 5 | `hull.rs` | `mesh.rs` (vertices) | Need convex hull for candidate direction generation. Pure geometry, good to validate independently. |
| 6 | `candidates.rs` | `hull.rs`, `scoring.rs` | Generate directions from hull normals, score them, rank. This is the minimum viable product. |
| 7 | `stability.rs` | `hull.rs`, `mesh.rs` | Reject candidates that would fall over. Critical for useful results. |
| 8 | `refine.rs` | `scoring.rs`, `candidates.rs` | Polish on top-K. Can ship without it in v1. |
| 9 | JS viewport | WASM output | Wire up after step 3 validates boundary. |
| 10 | JS thumbnail worker | Viewport | Separate concern, can build in parallel with steps 6-8. |
| 11 | JS favorites + export | Viewport + thumbnails | Last UX features. |

### Build Verification Milestones

1. **"Green smoke"** — `cargo build --target wasm32-unknown-unknown` succeeds with an empty `compute_orientations`. JS can call it.
2. **"Parsing"** — STL file bytes → Rust → parsed triangle count verified in JS console.
3. **"First score"** — Hardcoded up/down candidate returns `overhangPenalty`. Three.js viewport shows the model.
4. **"Ranked list"** — Hull-mode candidates returned and sorted. User can press next/prev.
5. **"No falling"** — Stability check active. Candidates that would tip over are marked unstable.
6. **"Shipping"** — All v1 features integrated: load → compute → view → yaw adjust → export.

## Concurrency Strategy

The performance profile of this application is well understood:
- Mesh sizes: 10K–500K triangles (typical resin minis: 50K–200K)
- Candidates: 20–200 (hull mode ~50, hull+sphere ~100)
- Scoring: O(n_triangles × n_candidates)
- Each candidate score: ~10 fused multiply-adds per triangle

### Benchmark Estimate

For a 100K-triangle mesh with 100 candidates:
- 100K × 100 = 10M triangle-score evaluations
- ~10 FMA per evaluation = ~100M float operations
- Rust WASM on modern hardware: **~30–80ms** (compiled, optimized)
- JS equivalent: **~300–800ms** (JIT-compiled, but boundary crossing overhead)

**Conclusion: For typical resin models, WASM computation completes in under 100ms. No threading needed for the core pipeline.**

### Concurrency Layers (in order of value)

#### Layer 1: WASM on main thread (v1, required)

Simplest possible approach. The WASM `compute_orientations()` call blocks the main thread for ~50-100ms. This is acceptable because:
- It happens once, immediately after file load
- The UI shows a loading spinner during computation
- Subsequent navigation (next/prev/thumbnails) is instant and doesn't call WASM

**Confidence: HIGH** — This is the standard pattern. No Web Worker complexity needed for v1.

#### Layer 2: Thumbnails in Web Worker (v3, nice-to-have)

Use a dedicated Worker for offscreen thumbnail rendering:

```
Main Thread                          Worker
─────────────                        ──────
Candidate[] ──postMessage──►      three.js scene for each candidate
  (data)                          Render to OffscreenCanvas
                               ◄──postMessage─── ImageBitmap[]
Display thumbnails
```

- Uses `OffscreenCanvas` API (Chrome 69+, Firefox 105+, Safari 16.4+)
- Worker loads its own three.js scene (model geometry shared via transferable)
- Each candidate thumbnail is rendered sequentially in the worker
- Result transferred back as `ImageBitmap` (zero-copy to canvas)

**Implementation pattern (from three.js manual / OffscreenCanvas docs):**
```typescript
// main.ts
const canvas = document.querySelector('#thumbnail-canvas');
const offscreen = canvas.transferControlToOffscreen();
const worker = new Worker('thumbnail-worker.ts', { type: 'module' });
worker.postMessage({ canvas: offscreen, candidates, geometry }, [offscreen]);

// thumbnail-worker.ts
import * as THREE from 'three';
self.onmessage = (e) => {
  const { canvas, candidates, geometry } = e.data;
  const renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
  const scene = new THREE.Scene();
  // ... render each candidate at fixed camera position
  // return ImageBitmap for each
};
```

**Confidence: HIGH** — Three.js has official OffscreenCanvas support with documented patterns. Proxy pattern needed for OrbitControls if interactivity is required in the worker (it's not needed here — thumbnails are static renders at fixed camera).

#### Layer 3: WASM in Web Worker (v2.x, if needed)

If computation time exceeds 300ms for very large meshes (500K+ triangles, 200+ candidates, refinement), move WASM to a Web Worker:

```
Main Thread                          Worker
─────────────                        ──────
ArrayBuffer ──postMessage──►      WASM compute_orientations()
                               ◄──postMessage─── Candidate[]
Display results
```

**When to add this:**
- Measured computation time > 200ms on target hardware (mid-range mobile)
- User tests show frustration with loading spinner duration
- Refinement iterations are added in v2 and increase compute time

**Implementation notes:**
- Worker does `import init, { compute_orientations } from './pkg/orient_core.js'`
- Worker instantiates WASM independently
- Data transferred via `postMessage` with transferable buffers
- No SharedArrayBuffer needed (single direction data flow, no concurrent access)
- No threading features needed in Rust (`wasm32-unknown-unknown` without atomics)

**Confidence: MEDIUM** — Standard pattern but adds build complexity. Not needed initially. Measure before implementing.

#### Layer 4: Rust threading (not recommended)

Using `wasm-bindgen-rayon` or `wasm-bindgen-spawn` for parallel scoring across cores:

**Decision: AVOID.** The complexity cost is not justified:
- Requires nightly Rust
- Requires `-C target-feature=+atomics` and recompiled std
- Requires `SharedArrayBuffer` + COOP/COEP HTTP headers
- Scoring is memory-bandwidth-bound (reading per-triangle data), not CPU-bound — parallelizing won't help proportionally
- The entire pipeline completes in <100ms on main thread anyway

**Confidence: HIGH** — Confirmed by WASM threading documentation and the performance profile of this specific workload.

## Patterns to Follow

### Pattern 1: Coarse-Grained Boundary

**What:** Design the WASM↔JS boundary so that a single call does all the work, rather than many small calls.

**When:** Always. Each boundary crossing has measurable overhead (marshalling arguments, copying memory).

**Example:**
```rust
// GOOD: one call
#[wasm_bindgen]
pub fn compute_orientations(positions: &[f32], config: &JsValue) -> JsValue { ... }

// BAD: many calls, each crossing the boundary
// pub fn parse_stl(bytes: &[u8]) -> TriangleCount
// pub fn compute_normals(triangles: &TriangleRef) -> Normals
// pub fn compute_hull(vertices: &[f32]) -> Hull
// ... each call from JS
```

The spec already follows this pattern. The entire pipeline stays inside WASM until the final result.

### Pattern 2: Borrow and Copy at Boundary

**What:** JavaScript owns the file bytes. WASM borrows them via `&[u8]`/`&[f32]` (zero-copy into WASM linear memory for numeric types). WASM returns ownership of the result via serialization (`JsValue`).

**When:** Always.
- Input: typed arrays → zero-copy borrow in WASM (wasm-bindgen marshals `Float32Array` to `&[f32]`)
- Output: serialize to plain objects → JS owns the data, WASM memory can be freed

**Example:**
```typescript
// JS: loads file, sends to WASM
const fileBytes = new Uint8Array(arrayBuffer);
const positions = new Float32Array(arrayBuffer);  // reinterpret STL triangle data

// WASM receives as &[f32], computes, returns serialized candidates
const candidates = compute_orientations(positions, config);

// Now WASM memory can be freed — candidates are plain JS objects
```

### Pattern 3: Module Isolation in Rust

**What:** Each Rust module (`mesh.rs`, `hull.rs`, etc.) is internally public within the crate but hidden behind `lib.rs`. Only `lib.rs` exports `#[wasm_bindgen]` items.

**When:** Always. This ensures the public API surface is minimal and explicit.

**Example:**
```rust
// lib.rs
mod mesh;
mod hull;
mod candidates;
mod scoring;
mod stability;
mod refine;
mod stl;

#[wasm_bindgen]
pub fn compute_orientations(positions: &[f32], config: &JsValue) -> JsValue { ... }

// Each module is pub(crate) — visible internally, hidden from WASM exports
```

### Pattern 4: Config Object Pattern

**What:** Accept a single `config` object (serialized from JS) rather than many individual parameters. This makes the API backward-compatible (adding new config fields doesn't break existing callers).

**When:** For any function with more than 2-3 configuration parameters.

**Example:**
```rust
#[derive(Deserialize)]
struct OrientConfig {
    mode: String,          // "hull" | "hull_plus_sphere"
    sphere_samples: Option<u32>,
    critical_angle_deg: f32,
    dedupe_angle_deg: f32,
    refine_iterations: u32,
    exclude_unstable: bool,
}

#[wasm_bindgen]
pub fn compute_orientations(positions: &[f32], config: &JsValue) -> JsValue {
    let config: OrientConfig = serde_wasm_bindgen::from_value(config.clone())
        .unwrap_or_else(|e| wasm_bindgen::throw_str(&e.to_string()));
    // ...
}
```

### Pattern 5: Vite + WASM Integration

**What:** Configure Vite to handle WASM imports correctly with `vite-plugin-wasm` and `vite-plugin-top-level-await`.

**When:** Required for bundler-based WASM consumption.

**Example:**
```typescript
// vite.config.ts
import { defineConfig } from 'vite';
import wasm from 'vite-plugin-wasm';
import topLevelAwait from 'vite-plugin-top-level-await';

export default defineConfig({
  plugins: [wasm(), topLevelAwait()],
});
```

This enables natural imports:
```typescript
import init, { compute_orientations } from 'orient-core';
await init();
```

**Confidence: HIGH** — Standard Vite + WASM setup documented by the Vite team.

### Pattern 6: Loading Flow

**What:** Async initialization pattern for WASM modules — init is called once, then all exports are synchronous.

**When:** Always with `wasm-bindgen` bundler target.

**Example:**
```typescript
// main.ts
let wasmReady = false;

async function initWasm() {
  await init();  // from the wasm-bindgen generated module
  wasmReady = true;
}

async function onFileLoad(file: File) {
  if (!wasmReady) await initWasm();
  const buffer = await file.arrayBuffer();
  const positions = new Float32Array(buffer);
  const candidates = compute_orientations(positions, config);
  displayCandidates(candidates);
}
```

## Anti-Patterns to Avoid

### Anti-Pattern 1: Fine-Grained WASM API

**What:** Exporting many small functions (e.g., `parse_stl()`, `compute_normals()`, `generate_candidates()`, `score_candidate()`) and calling them in sequence from JS.

**Why bad:** Each crossing adds overhead. JS orchestrates the pipeline, adding latency between steps. The "WASM doesn't help" benchmark case.

**Instead:** Single `compute_orientations()` function. Let WASM orchestrate the pipeline internally.

### Anti-Pattern 2: Returning `#[wasm_bindgen]` Struct Handles for Result Sets

**What:** Returning a `Vec<Candidate>` where `Candidate` is a `#[wasm_bindgen]` struct, creating JS class instances that hold pointers to WASM memory.

**Why bad:** Each access to a property crosses the boundary. JS must manually free each handle. For 100 candidates in a list view where the user navigates through them, you'd either hold 100 handles (memory management burden) or create JS wrappers anyway (double representation).

**Instead:** Serialize to plain objects with `serde-wasm-bindgen`. WASM memory is released after the call. JS has zero-overhead access to properties.

### Anti-Pattern 3: WASM Threading Prematurely

**What:** Setting up `wasm-bindgen-rayon`, SharedArrayBuffer, COOP/COEP headers, nightly Rust, and recompiled std before measuring whether the scoring is fast enough.

**Why bad:** The entire compute pipeline for realistic meshes is estimated at 30-100ms. Adding threading multiplies the build complexity by 10x for no perceptible benefit. The COOP/COEP headers also disable some CDN features and may cause issues with third-party resources.

**Instead:** Measure first. If <200ms, ship on main thread. Only reach for threading if >500ms on a representative mesh.

### Anti-Pattern 4: ASCII STL in WASM

**What:** Supporting ASCII STL parsing in the WASM module, which requires text parsing, line splitting, and more complex error handling.

**Why bad:** ASCII STL is rare in production (slicers output binary). Parsing ASCII adds ~200 lines and increased WASM binary size. The text parsing also involves more allocations and string operations, increasing the WASM binary through Rust's formatting machinery.

**Instead:** Parse binary only in WASM (stl-io handles it cleanly). If ASCII support is needed later, do it in JS or confirm with a spike first. The `orient-spec.md` defers this with "confirm if real files are always binary."

### Anti-Pattern 5: Re-parsing STL on Export

**What:** When exporting an oriented STL, reading the original file from disk again or parsing the STL from the ArrayBuffer a second time.

**Why bad:** The original triangle data exists in the three.js BufferGeometry and in the original ArrayBuffer. Re-parsing wastes time.

**Instead:** Read vertices directly from the three.js `BufferGeometry` or cache the parsed triangle data in JS memory. Apply the quaternion to each vertex and write the rotated STL. This is O(n) in triangles and avoids any cross-boundary calls.

## Scalability Considerations

| Concern | At 10K triangles / 50 candidates | At 200K triangles / 100 candidates | At 500K triangles / 200 candidates (with refinement) |
|---------|----------------------------------|-------------------------------------|------------------------------------------------------|
| WASM compute time | ~5ms | ~50ms | ~300ms |
| WASM binary size | ~300KB compressed | ~300KB (same code, data scales with input, not binary) | ~300KB |
| JS memory (candidate list) | ~50KB | ~100KB | ~200KB |
| JS memory (geometry) | ~500KB | ~5MB | ~15MB |
| JS memory (thumbnails) | ~2MB (20 at 320×240) | ~2MB (same, top-N throttled) | ~2MB |
| Thumbnail render time | ~50ms per candidate | ~200ms per candidate | ~500ms per candidate |
| Loading spinner UX | Barely noticeable | Brief (<1s) | Acceptable (show progress) |

**When compute exceeds ~200ms:** Move WASM to Web Worker. The main thread stays responsive.
**When triangle count exceeds 500K:** Throttle thumbnail generation to top 20 candidates. Show thumbnails progressively.
**When candidate count exceeds 200:** Pre-filter candidates via dedupe more aggressively, or show only top-K by default with "show all" option.

## Source Code Layout

```
orient/
├── core/                          # Rust crate
│   ├── Cargo.toml                 # cdylib + rlib
│   ├── Cargo.lock
│   └── src/
│       ├── lib.rs                 # #[wasm_bindgen] exports ONLY here
│       ├── stl.rs                 # STL binary parser (stl-io wrapper or vendored)
│       ├── mesh.rs                # Per-triangle precomputation
│       ├── hull.rs                # Quickhull (vendored, ~300 lines)
│       ├── candidates.rs          # Candidate generation + dedupe + yaw computation
│       ├── scoring.rs             # Overhang penalty scoring (S² space)
│       ├── stability.rs           # Footprint + CoM + point-in-polygon
│       └── refine.rs             # S² hill-climbing
├── web/                           # JS/TS app (Vite)
│   ├── index.html
│   ├── vite.config.ts             # wasm plugin + top-level-await plugin
│   ├── package.json
│   ├── tsconfig.json
│   └── src/
│       ├── main.ts                # App entry point, WASM init, file input
│       ├── loadSTL.ts             # FileReader → ArrayBuffer → WASM bridge
│       ├── viewport.ts            # three.js scene, next/prev navigation
│       ├── thumbnails.ts          # OffscreenCanvas thumbnail worker
│       ├── favorites.ts           # IndexedDB persistence
│       ├── exportSTL.ts           # fflate ZIP export
│       ├── types.ts               # TS types matching WASM output
│       ├── yaw-dial.ts            # Circular yaw dial UI
│       └── workers/
│           └── thumbnail.worker.ts # Web Worker for offscreen rendering
```

## Sources

- **wasm-bindgen Guide** — Official documentation on exporting/importing, boundary types, config. https://rustwasm.github.io/docs/wasm-bindgen/
- **Rust for TS/JS Developers (2026)** — WASM module structure, boundary cost analysis, performance best practices. https://rs4ts.dev/19-wasm/
- **Three.js OffscreenCanvas Manual** — Official three.js guide for Worker-based rendering with OffscreenCanvas. https://threejs.org/manual/en/offscreencanvas.html
- **wasm-bindgen Raytrace Example** — Threading with SharedArrayBuffer in WASM. Confirms complexity of threaded approach. https://rustwasm.github.io/docs/wasm-bindgen/examples/raytrace.html
- **Vite WASM Plugin** — Official Vite plugin for WASM module support. https://github.com/nicolo-ribaudo/vite-plugin-wasm
- **wasm-bindgen-spawn crate docs** — Threading library for wasm32-unknown-unknown. Confirms nightly + atomics requirement. https://docs.rs/wasm-bindgen-spawn/latest/wasm_bindgen_spawn/
