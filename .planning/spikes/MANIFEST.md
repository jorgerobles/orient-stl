# Spike Manifest

## Idea

Validate scoring heuristics for resin-printing orientation ranking. Spikes explore
the core computation (STL parsing in WASM) and the ranking algorithms (overhang,
footprint, cross-section composites) that make orient-stl useful for resin printers.

## Requirements

- STL parsing must work in WASM browser context (`wasm32-unknown-unknown`)
- Must support binary STL (primary format); ASCII STL is optional
- API must accept `&[u8]` / `Cursor<&[u8]>` (bytes from JS `FileReader`)
- Zero or minimal wasm binary size overhead
- Scoring must be resin-aware: cross-section / peel-force is the dominant failure
  mode, not gravity-driven overhang (FDM thinking)
- All scoring lives in the Rust/WASM core (WASM-first decision); JS only displays

## Spikes

| # | Name | Type | Validates | Verdict | Tags |
|---|------|------|-----------|---------|------|
| 001 | stl-io-wasm | standard | "Given an STL binary file buffer in WASM memory, when parsed with stl-io crate compiled to wasm32-unknown-unknown, then it produces a valid IndexedMesh without pulling std::fs or blocking I/O" | PARTIAL ⚠ | rust, wasm, stl, parsing |
| 002 | scoring-composite-harness | standard | "Given a mesh + candidate directions, when scored with variable weights of (overhang, footprint, max-cross-section), then the ranking differs meaningfully from overhang-only and the composite produces lower peel-force orientations — verifiable via the harness on real fixtures" | VALIDATED ✓ | rust, scoring, resin, resin-physics, heuristics |
