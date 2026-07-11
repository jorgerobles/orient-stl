# Domain Pitfalls

**Domain:** Resin 3D printing auto-orientation with Rust WASM + three.js
**Researched:** 2026-07-11

## Critical Pitfalls

Mistakes that cause rewrites or major issues.

### Pitfall 1: Leaking WASM Memory via `#[wasm_bindgen]` Struct Handles

**What goes wrong:** Returning `Vec<Candidate>` where `Candidate` is a `#[wasm_bindgen]` struct creates JS class instances that hold pointers to WASM linear memory. Each one must be `.free()`'d or the memory leaks. For 100 candidates, that's 100 manual `.free()` calls the JS side must remember.

**Why it happens:** `#[wasm_bindgen]` structs are the most natural way to export structured data. It feels like returning a normal object. The memory management requirement is not obvious.

**Consequences:** WASM memory grows unboundedly (leak). Or worse, the WebAssembly.Instance's memory runs out silently on repeated file loads. Or JS tries to access a freed handle and gets garbage data.

**Prevention:** Use `serde-wasm-bindgen` to serialize `Vec<Candidate>` to a `JsValue` containing plain JS objects. No handles, no `.free()`, no lifetime coupling. WASM memory is released when the function returns.

**Detection:** Use `console.log(wasmModule.__wbg_wasm.toString())` to check for orphaned handles. Use the browser's memory profiler to look for growing WASM memory between file loads. Or check: if `Candidate` has a `.free()` method → you're at risk.

### Pitfall 2: Premature WASM Threading

**What goes wrong:** Setting up `wasm-bindgen-rayon`, SharedArrayBuffer, nightly Rust, COOP/COEP headers, and recompiled standard library for threading — before measuring whether the scoring is fast enough on the main thread.

**Why it happens:** WASM + threading sounds like it should be faster. The compute pipeline (scoring ~100 candidates against ~100K triangles) sounds like a lot of work.

**Consequences:**
- Nightly Rust toolchain required (unstable)
- COOP/COEP headers break some CDN resources and third-party scripts
- `SharedArrayBuffer` requires specific HTTP headers — blocks local dev without a configured server
- Build complexity increases 10x for a feature that may not help (scoring is memory-bandwidth bound, not CPU-bound)
- WASM binary size increases due to atomics support code

**Prevention:** Benchmark the single-threaded WASM pipeline on representative meshes (50K–500K triangles) before considering threading. If <200ms on mid-range mobile, ship on main thread.

**Detection:** Measure `performance.now()` around the `compute_orientations()` call on target hardware. If >500ms, evaluate Worker-based approach (not SharedArrayBuffer threading).

### Pitfall 3: Calling WASM Functions Before `init()` Resolves

**What goes wrong:** `compute_orientations()` is called before the WASM module is fully instantiated. The module is imported as an ES module, but the WASM binary must be fetched and instantiated asynchronously.

**Why it happens:** The generated JS glue's `init()` is async, but the rest of the module looks like a normal ES module. It's easy to forget the `await init()` step.

**Consequences:** Hard-to-debug errors: `TypeError: compute_orientations is not a function` or `RuntimeError: unreachable executed`.

**Prevention:** The single most well-documented pitfall in wasm-bindgen. Always `await init()` before using any exports. Store a `wasmReady` flag. Consider `init()` at app startup before the file input is revealed.

**Detection:** Check whether `init()` was called in the console. The error message usually mentions "not a function."

### Pitfall 4: Fine-Grained WASM Boundary

**What goes wrong:** Exporting 10+ small WASM functions (parse, compute normals, generate hull, generate candidates, score each candidate, check stability, etc.) and calling them from JS in sequence.

**Why it happens:** It feels modular and testable. Each function does one thing.

**Consequences:** Each call crosses the JS↔WASM boundary, adding marshalling overhead. JS orchestrates the pipeline, adding JavaScript execution time between steps. WASM underperforms expectations because the boundary cost dominates.

**Prevention:** A single `compute_orientations()` function. WASM does everything internally, crossing the boundary only to receive input and return output. Test individual modules internally with `cargo test`.

## Moderate Pitfalls

### Pitfall 5: Using three.js STLLoader for Parsing, Then Copying to WASM

**What goes wrong:** Loading the STL with three.js STLLoader (which creates a BufferGeometry), then extracting the position attribute and copying it into WASM memory.

**Prevention:** Pass the raw file bytes directly to WASM. Let Rust parse the STL from the raw `&[u8]`. This avoids an unnecessary JS-side parse and the extra copy from BufferGeometry to WASM.

### Pitfall 6: Not Checking Rust Panic Messages

**What goes wrong:** Rust panics in WASM produce the unhelpful `RuntimeError: unreachable executed` in the browser console, with no indication of what went wrong.

**Prevention:** Use `console_error_panic_hook::set_once()` in a `#[wasm_bindgen(start)]` function. Gate it behind a `cfg(debug_assertions)` or Cargo feature so it's only included in dev builds.

### Pitfall 7: Building Without `rlib` Crate Type

**What goes wrong:** `crate-type = ["cdylib"]` works for WASM builds but `cargo test` fails because tests need an `rlib` target.

**Prevention:** Always use `crate-type = ["cdylib", "rlib"]`. The `rlib` costs nothing in the WASM binary and makes local testing possible.

### Pitfall 8: Over-Engineering the Config Object

**What goes wrong:** Building a complex nested config validation system, enum parsing, or custom deserialization when a simple `serde_json::from_value` or `serde_wasm_bindgen::from_value` would work.

**Prevention:** Use `#[derive(Deserialize)]` on a flat config struct with `#[serde(default)]` for optional fields. The JS side passes a simple object literal. Add fields as needed — serde handles backward compatibility.

### Pitfall 9: Not Serving WASM with Correct MIME Type

**What goes wrong:** The browser refuses to load the `.wasm` file because the server returns `application/octet-stream` instead of `application/wasm`.

**Prevention:** Vite dev server handles this automatically. For production, ensure the web server (nginx, Cloudflare, etc.) is configured to serve `.wasm` with `Content-Type: application/wasm`.

## Minor Pitfalls

### Pitfall 10: Ignoring `duplicate` in stl-io

stl-io's `read_stl()` returns a `Normal` for each triangle. The normal can be invalid (zero-length). Validate and skip/recalculate normals rather than assuming they're correct.

### Pitfall 11: Wrong Float Precision

STL uses f32 (32-bit floats). Rust's default f64 is perfectly fine for computation, but when interfacing with three.js (which uses f32 for BufferGeometry), ensure the quaternions and positions stay in f32 range. No precision issues expected.

### Pitfall 12: Failing to Handle `deduplicateAngle`

Candidates at similar directions produce almost identical scores. Without deduplication, the ranked list shows the same orientation 5 times with slightly different scores. The `dedupeAngleDeg` parameter (suggested: 2-5°) is critical for UX quality.

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|---------------|------------|
| Rust/WASM setup (Phase 1) | Missing `cdylib` crate type / forgetting `await init()` | Add to boilerplate checklist. Generate scaffolding from known-good template. |
| Candidate generation (Phase 1) | No deduplication, causing duplicate orientations | Make dedupe required, not optional. Default to 3°. |
| Stability check (Phase 1) | Numerical edge cases in point-in-polygon (vertex exactly on edge, concave hull projection) | Use winding number or ray casting with epsilon tolerance. Test with known unstable orientations. |
| Viewport (Phase 1) | three.js Loader overhead — loading STLLoader when WASM already parsed | Use a minimal JS STL parser for viewport display, or pass WASM-parsed data back to JS for display. Decision needed. |
| Yaw dial (Phase 2) | Rotating calipers implementation bugs in 2D convex hull yaw computation | Validate against brute-force bounding-box enumeration for first 100 orientations. |
| Thumbnails (Phase 3) | OffscreenCanvas not supported in older browsers | Provide fallback: render thumbnails on main thread with a visible canvas. |
| ZIP export (Phase 3) | Memory pressure from multiple large STL blobs | Stream/cascade the ZIP generation. fflate supports asynchronous compression. |

## Sources

- wasm-bindgen Guide — Common pitfalls section: https://rustwasm.github.io/docs/wasm-bindgen/
- Rust for TS/JS Developers — WASM performance and boundary cost analysis: https://rs4ts.dev/19-wasm/09-performance/
- Three.js OffscreenCanvas documentation — Browser support notes: https://threejs.org/manual/en/offscreencanvas.html
- wasm-bindgen Raytrace example — Threading caveats: https://rustwasm.github.io/docs/wasm-bindgen/examples/raytrace.html
- Community knowledge: Quickhull implementation edge cases, point-in-polygon numerical stability
