---
name: spike-findings-orient-stl
description: Implementation blueprint from spike experiments. Requirements, proven patterns, and verified knowledge for building orient-stl. Auto-loaded during implementation work.
---

<context>
## Project: orient-stl

Resin printing orientation tool — load an STL, compute a ranked list of candidate orientations minimizing overhangs/supports, navigate in a single viewport, generate PNG previews, mark favorites, and export oriented STLs.

Core calculation in Rust → WASM. UI in JS/TS + three.js (Vite). Favorites persisted in IndexedDB.

Spike sessions wrapped: 2026-07-11
</context>

<requirements>
## Requirements

- STL parsing must work in WASM browser context (`wasm32-unknown-unknown`)
- Must support binary STL (primary format); ASCII STL is optional
- API must accept `&[u8]` / `Cursor<&[u8]>` (bytes from JS `FileReader`)
- Zero or minimal wasm binary size overhead
</requirements>

<findings_index>
## Feature Areas

| Area | Reference | Key Finding |
|------|-----------|-------------|
| STL Parsing | references/stl-parsing.md | Use `stl-io` v0.11.0 — zero-dependency, takes `impl Read`, pure parsing, strongly compatible with wasm32-unknown-unknown |

## Source Files

Original spike source files are preserved in `sources/` for complete reference.
</findings_index>

<metadata>
## Processed Spikes

- 001-stl-io-wasm
</metadata>
