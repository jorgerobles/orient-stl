---
title: Spike stl-io WASM compatibility
date: 2026-07-11
priority: high
context: Decidir si usar stl-io crate o vender parser STL binario (~40 líneas)
---

**Goal**: Confirm stl-io compiles cleanly to `wasm32-unknown-unknown` without pulling filesystem dependencies or requiring `std::fs`.

**Method**:
1. `cargo new orient-stl-spike && cd orient-stl-spike`
2. Add `stl-io` to Cargo.toml
3. Add `wasm-pack build --target web` to verify
4. Check `cargo tree` for transitive deps that look problematic (anything requiring `std::fs`, `std::net`, etc.)

**Success**: Compiles without errors, `cargo tree` shows only parsing/IO-free deps.
**Failure**: Any `std::fs` or blocking I/O assumption → vendor ~40-line binary STL parser in `core/src/stl.rs`.
