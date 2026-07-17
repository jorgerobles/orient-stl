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

<hard_rules>
## WASM Rebuild Rule

After ANY change to `core/src/*.rs`, WASM MUST be rebuilt before testing or committing:

```bash
wasm-pack build core --target bundler --out-dir web/pkg
```

The prebuilt `.wasm` at `web/pkg/` is a stale build artifact that does NOT auto-sync with Rust source. Forgetting this causes silent runtime errors ("Unknown mode", missing exports, wrong signatures). No Rust edit is complete until `wasm-pack build` succeeds and `web/pkg/` is updated.

## Regression & Performance Verification Rule

Any change to the mesh processing pipeline (`repair_mesh`, `normalize_winding`, `fill_holes`, `weld_vertices`, or the `prepare_data_native_with_repair` orchestrator) MUST be verified for regressions in candidate count, score quality, and latency before merging.

**Minimum verification against a known-good STL baseline:**
- Number of candidates must not decrease by more than 10%
- Composite scores of top-3 candidates must not regress by more than 5%
- End-to-end WASM processing time must not increase by more than 20%

**Baseline reference** (measured 2026-07-17, release build, worm & broken STL, repair disabled by default):
| Mesh | Tris | Candidates | Boundary Edges | Time (s) |
|------|------|-----------|----------------|----------|
| worm.stl (no repair) | 499K | 408 | 1720 | 12.4 |
| worm.stl (repair) | 499K | 414 | 1719 | 5.8 |
| broken.stl (no repair) | 449K | 414 | 136,539 | 10.1 |
| broken.stl (repair) | 478K | 408 | 77,235 🔸 | 18.6 |

⚠️ **KNOWN BUG FIXED 2026-07-17**: `prepare_data_native` was passing `DEFAULT_WELD_EPSILON` (1e-5) instead of 0.0, silently welding all STLs even in "baseline" mode. WASM JS defaults also had `weldEpsilon: 1e-5` and `maxHoleEdges: 64`, activating the full repair pipeline on every load. This reduced candidate count (e.g. 10→3 on "uzi jesus") and changed scores. Fix: `prepare_data_native` now passes `weld_epsilon: 0`, and JS defaults are `weldEpsilon: 0, maxHoleEdges: 0`.

**Repair is opt-in.** When `weldEpsilon > 0` and/or `maxHoleEdges > 0`, the pipeline runs `repair_mesh → normalize_winding → weld_vertices → repair_mesh → fill_holes → precompute_mesh`. Use only for meshes with known issues (many boundary edges, non-manifold geometry). Performance on 500K tris: +84% on broken.stl.

Record a new baseline after each verified change. If a change intentionally reduces candidates or scores (e.g. removing false positives), document the trade-off in the commit message.

Changes that only affect UI/JS/TS code (no Rust/WASM rebuild) are exempt from this rule.
</hard_rules>

<metadata>
## Processed Spikes

- 001-stl-io-wasm
</metadata>
