# Spike Manifest

## Idea

Validate whether `stl-io` Rust crate compiles cleanly to `wasm32-unknown-unknown` for parsing binary STL files in a browser WASM context, or whether we need to vendor a minimal parser.

## Requirements

- STL parsing must work in WASM browser context (`wasm32-unknown-unknown`)
- Must support binary STL (primary format); ASCII STL is optional
- API must accept `&[u8]` / `Cursor<&[u8]>` (bytes from JS `FileReader`)
- Zero or minimal wasm binary size overhead

## Spikes

| # | Name | Type | Validates | Verdict | Tags |
|---|------|------|-----------|---------|------|
| 001 | stl-io-wasm | standard | "Given an STL binary file buffer in WASM memory, when parsed with stl-io crate compiled to wasm32-unknown-unknown, then it produces a valid IndexedMesh without pulling std::fs or blocking I/O" | PARTIAL ⚠ | rust, wasm, stl, parsing |
