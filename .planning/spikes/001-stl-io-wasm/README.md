---
spike: 001
name: stl-io-wasm
type: standard
validates: "Given an STL binary file buffer in WASM memory, when parsed with stl-io crate compiled to wasm32-unknown-unknown, then it produces a valid IndexedMesh without pulling std::fs or blocking I/O"
verdict: PARTIAL
related: []
tags: [rust, wasm, stl, parsing]
---

# Spike 001: stl-io WASM Compatibility

## What This Validates

Can the `stl-io` Rust crate be compiled to `wasm32-unknown-unknown` and used in a browser WASM context to parse binary STL files from a `&[u8]` buffer, or do we need to vendor a minimal parser?

## Research

### stl-io v0.11.0 (latest, March 2026)

| Property | Value |
|----------|-------|
| Version | 0.11.0 |
| Dependencies | **Zero** — pure Rust stdlib, no external deps |
| Total downloads | ~3.8M |
| License | MIT |
| API | `read_stl(impl Read)`, `create_stl_reader(impl BufRead)`, `write_stl(impl Write)` |
| Features | Auto-detects binary vs ASCII STL, indexed mesh output, normal + vertex types |

### WASM Compatibility Analysis

Three independent factors all point to clean compatibility:

1. **Zero dependencies**: No transitive dependency risk. No optional features that could pull in `std::thread`, `std::fs`, rayon, BLAS, or any wasm-incompatible code.

2. **API surface**: All public functions take `impl Read` / `impl BufRead` / `impl Write` — generic over the data source. The caller provides the byte stream. In a WASM context, `std::io::Cursor<&[u8]>` implements `Read` + `BufRead`, which works on `wasm32-unknown-unknown` without issues.

3. **Stdlib compatibility**: `wasm32-unknown-unknown` supports `core`, `alloc`, and most of `std` including `std::io::Read`/`Write` traits. Only filesystem, threading, and networking APIs fail at runtime. stl-io doesn't use any of those internally — it's pure in-memory data parsing.

### Chosen Approach: Use stl-io

Use stl-io as a dependency. If for any reason the crate proves problematic at compile time (e.g., a future version adds a problematic dep), fall back to a vendored ~40-line binary STL parser. The binary STL format is:

```
[80-byte header (ignored)]
[u32 triangle_count]
for each triangle:
  [f32 nx, ny, nz] = normal
  [f32 v1x, v1y, v1z] = vertex 1
  [f32 v2x, v2y, v2z] = vertex 2
  [f32 v3x, v3y, v3z] = vertex 3
  [u16 attribute_byte_count (usually 0)]
= 50 bytes per triangle after header + count
```

Note: binary STL stores normals redundantly per triangle. The spec's `mesh.rs` precompute step discards these and recomputes from geometry.

## How to Run

```bash
# Requires Rust toolchain installed (rustup + wasm32-unknown-unknown target)
rustup target add wasm32-unknown-unknown
cargo add stl_io --vers 0.11.0
cargo build --target wasm32-unknown-unknown
```

Then integrate into the `core/` crate with:
```rust
use stl_io::read_stl;
use std::io::Cursor;

let mesh = read_stl(&mut Cursor::new(wasm_memory_slice)).unwrap();
```

## What to Expect

- `cargo build --target wasm32-unknown-unknown` succeeds without errors
- `wasm-pack build` produces a valid `.wasm` binary
- No runtime panics when parsing a valid binary STL from a `Cursor<&[u8]>`

## Environment Constraints

This spike was researched in an environment without a Rust toolchain. The analysis above is based on:
- Crate API documentation (docs.rs)
- Zero dependency check (crates.io API)
- `wasm32-unknown-unknown` stdlib compatibility documentation
- Knowledge of the STL binary format

A live compilation test should be performed once the Rust toolchain is available in the development environment. The analysis confidence is **high** (≥90%) but not fully verified.

## Investigation Trail

1. Checked crates.io API — stl-io v0.11.0 has 0 dependencies
2. Reviewed docs.rs — API takes `impl Read`, no internal filesystem usage
3. Checked wasm32-unknown-unknown platform support — `std::io::Read`/`Write`/`Cursor` all work
4. Confirmed binary STL format is straightforward (50 bytes/triangle after header) — vendor path is trivial if needed
5. Attempted `wasm-pack build` — not possible without Rust toolchain installed

## Results

**Verdict: PARTIAL ⚠**

Strong evidence that stl-io compiles cleanly to wasm32-unknown-unknown, but not fully verified by a live compilation.

| Check | Status | Evidence |
|-------|--------|----------|
| Zero dependencies | ✓ Confirmed | crates.io API: `{"dependencies":[]}` |
| Uses `impl Read` not `std::fs` | ✓ Confirmed | docs.rs API docs show `Read` generics |
| ASCII STL support included | ✓ Free | Auto-detection built into crate |
| Binary STL format understood | ✓ Verified | Roundtrip test via `stl-format-ref.js` (184 bytes for 2 tris) |
| Compiles to wasm32 | ⚠ Not tested | Requires Rust toolchain |
| Binary size impact | ⚠ Not measured | Stl-io is small, but exact bytes TBD |

**Node.js proof-of-concept**: `stl-format-ref.js` validates the binary STL wire format — 80-byte header, u32 count, N×50-byte triangles. This is the format the vendor parser would handle. Run: `node .planning/spikes/001-stl-io-wasm/stl-format-ref.js`.

**Recommendation**: Use stl-io v0.11.0. If the live compilation test (blocked by missing Rust toolchain) reveals issues, fall back to a vendored ~40-line parser for binary STL only (no ASCII support in vendor path). The vendor parser would be trivial but loses ASCII STL support, which is a secondary concern.
