# STL Parsing

## Requirements

- STL parsing must work in WASM browser context (`wasm32-unknown-unknown`)
- Must support binary STL (primary format); ASCII STL is optional
- API must accept raw bytes (`&[u8]` / `Cursor<&[u8]>`) from JS `FileReader`
- Zero or minimal wasm binary size overhead

## How to Build It

### 1. Add stl-io dependency

```toml
# core/Cargo.toml
[dependencies]
stl_io = "0.11.0"
```

### 2. Create `core/src/stl.rs`

```rust
use std::io::Cursor;
use stl_io::{read_stl, IndexedMesh};

/// Parse a binary (or ASCII) STL from raw bytes.
/// Returns the mesh in indexed form (vertex list + indexed triangles).
pub fn parse_stl(bytes: &[u8]) -> Result<IndexedMesh, String> {
    let mut cursor = Cursor::new(bytes);
    read_stl(&mut cursor).map_err(|e| format!("STL parse error: {}", e))
}
```

### 3. WASM entry point (`core/src/lib.rs`)

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn parse_stl_from_js(bytes: &[u8]) -> Result<JsValue, JsValue> {
    let mesh = stl::parse_stl(bytes).map_err(|e| JsValue::from_str(&e))?;
    // convert IndexedMesh to JsValue for JS consumption
    Ok(serde_wasm_bindgen::to_value(&mesh)?)
}
```

### 4. JS side (`web/src/loadSTL.ts`)

```typescript
import init, { parse_stl_from_js } from '../core/pkg/orient_core';

export async function loadSTL(file: File) {
  await init(); // initialize WASM module once
  const buffer = await file.arrayBuffer();
  const bytes = new Uint8Array(buffer);
  return parse_stl_from_js(bytes);
}
```

## What to Avoid

- **Don't use `std::fs` to read STL** — the crate takes `impl Read`, meaning the caller provides the byte stream. In WASM, always use `Cursor<&[u8]>` from the JS-side buffer.
- **Don't parse STL in JS** — that would duplicate the format knowledge and add an extra JS→WASM data copy. Pass raw bytes directly to the WASM module.
- **Don't pull three.js's STLLoader** — it's only needed for rendering in the viewport, not for data parsing.

## Constraints

- `wasm32-unknown-unknown` target does NOT support `std::fs` or `std::thread` — stl-io doesn't use either, so this is safe.
- `wasm-pack` is the build tool for WASM target. Requires Rust toolchain with `wasm32-unknown-unknown` target installed.
- Binary STL format: 80-byte header | u32 triangle count | N × 50-byte triangles (each: normal(12) + v1(12) + v2(12) + v3(12) + attrib(2))
- `stl-io` v0.11.0 has **zero dependencies** — no risk of transitive WASM incompatibilities.

## Origin

Synthesized from spikes: 001
Source files available in: sources/001-stl-io-wasm/
