---
title: Spike stl-io WASM compatibility
date: 2026-07-11
resolved: 2026-07-11
priority: high
outcome: confirmed — stl_io compiles cleanly to wasm32-unknown-unknown
---

**Goal**: Confirm stl-io compiles cleanly to `wasm32-unknown-unknown` without pulling filesystem dependencies or requiring `std::fs`.

## Finding: CONFIRMED — stl_io is WASM-safe

The spike succeeded. `stl_io = "0.11"` is in `core/Cargo.toml` and the crate compiles
and runs cleanly under `wasm32-unknown-unknown`. The production WASM binary
(`web/pkg/orient_core_bg.wasm`, 132KB) parses STL bytes via stl_io using a `Cursor<&[u8]>`
— no filesystem access, no blocking I/O.

**Evidence:**
- `core/Cargo.toml` lists `stl_io = "0.11"`
- `cargo check --target wasm32-unknown-unknown` passes
- `wasm-pack build` produces a working binary
- `core/src/stl.rs::parse_stl` calls `stl_io::read_stl(&mut cursor)` on an in-memory buffer
- The running app at localhost:5173 successfully loads and parses STL files

No vendored parser needed — stl_io's zero-dependency, `impl Read`-based API is exactly
what WASM requires.

## Recalled Direction: Use WASM for computation (WebGPU aspirational)

The spike's original framing drifted during implementation. Restating the architectural
direction explicitly:

- **WASM (Rust → wasm-bindgen) is the computation core.** All expensive geometry work —
  STL parsing, mesh precompute, convex hull, candidate generation, deduplication — lives
  in Rust and runs in WASM. This is confirmed and working.

- **Scoring/stability/yaw currently run in JS Web Workers** (architecture drift from the
  original plan — see 01-02-SUMMARY). This was a pragmatic choice for iteration speed.
  If JS scoring becomes a bottleneck on large meshes, the direction is to **move it back
  into the WASM core**, not to optimize the JS path.

- **WebGPU is the performance frontier for future phases.** If/when per-candidate
  evaluation or rendering needs GPU-class throughput (e.g. thumbnail rendering at scale,
  dense Fibonacci-sphere sampling in Phase 3, or real-time heatmap in Phase 3), the
  direction is a WebGPU compute pipeline — not more WASM threads and not more JS.
  WebGPU is aspirational for v2/v3, not v1.

**Decision logged:** WASM-first for computation; WebGPU is the documented upgrade path.
Do not add JS-side SIMD, WASM threads (SIMD/atomics target), or Node-native deps.
