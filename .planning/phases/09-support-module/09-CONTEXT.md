# Phase 9: Support Module - Context

**Gathered:** 2026-08-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Implement support generation as an independent WASM module: island detection (2D slice rasterization), volume-aware classification (Light/Medium/Heavy), Poisson-disk contact placement, and line-connected raft generation. The module is the 5th WASM binary in the Unix-style architecture, receives flat `&[f32]` arrays from orient, and produces `SupportResult` with supports, raft, and metrics.

**In scope:**
- `crates/support/` with island.rs, volume.rs, placement.rs, raft.rs, types.rs
- Island detection via 2D slice rasterization + connected components
- Volume classification (ray-cast mass-above, 3 tiers)
- Variable-density Poisson-disk contact placement with edge seeding
- Line-connected raft: convex hull + Delaunay + MST
- WASM bindings: `generate_supports()` export
- Pipeline integration: support runs after orient scoring
- Ground-truth unit tests (flat plate, cube, cylinder)

**Out of scope:**
- UI integration (Phase 10: toggle, config panel, viewport rendering)
- Export with supports baked in (Phase 10)
- User-editable profile creation UI (future phase)
- WebGPU acceleration (aspirational)

</domain>

<decisions>
## Implementation Decisions

### Performance Budget
- **D-01:** Use BVH acceleration + coarse grid pass — build BVH over triangles for ray queries, rasterize at coarser grid (1mm) for island detection, refine to 0.5mm only for contact placement. This avoids O(tris × layers) full-resolution rasterization.
- **D-02:** Per-stage progress reporting: 4 stages (islands → classification → placement → raft). Matches pipeline.ts `onProgress` pattern.
- **D-03:** Keep HANDOFF caps: max 500 islands, max 10000 contacts. Excess truncated with warning.

### Degenerate Mesh Handling
- **D-04:** Filter + warn — skip zero-area triangles during rasterization, emit warning count. Don't crash, don't fabricate data.
- **D-05:** Density cap per island at ~200 contacts. Large overhangs get wider spacing, not infinitely dense supports.

### Algorithm Tuning Parameters
- **D-06:** Volume thresholds locked as defaults: Light <50mm³, Medium 50-500mm³, Heavy >500mm³. Exposed in SupportConfig for Phase 10 UI override.
- **D-07:** Poisson-disk spacing ranges locked as defaults: Light 2.5-6mm, Medium 2-5mm, Heavy 1.5-3.5mm.
- **D-08:** Penetration depths locked: Light 0.2mm, Medium 0.3mm, Heavy 0.4mm. Tip diameters (0.25/0.40/0.80mm) deferred to Phase 10 UI — they're user-preference, not geometrically constrained.
- **D-09:** Curvature thresholds for circular deformity locked: cos>0.7 → Heavy (cylinder bottom), 0.3<cos<0.7 → Medium (sides), cos<0.3 → Light (top).
- **D-10:** Raft dimensions locked as defaults: 1.0mm thickness, 1.5mm line width.
- **D-11:** Rasterization grid cell size locked at 0.5mm (10x standard layer height, good accuracy/speed balance).

### Architecture (from HANDOFF, already decided)
- **D-12:** Support is an independent WASM binary — no dependency on orient or geometry-kernel crates.
- **D-13:** WASM API: `generate_supports(positions, normals, areas, direction, config) -> SupportResult` (JSON via serde_wasm_bindgen).
- **D-14:** Support worker (`support.worker.ts`) lazy-loads WASM, handles messages, posts results.
- **D-15:** Pipeline integration: support stage runs after orient, gated by `config.generateSupports`.

### the agent's Discretion
- Whether to implement BVH as a simple bounding-box tree or use an existing crate (e.g., `bvh` crate)
- Exact Bowyer-Watson Delaunay implementation details (~150 lines, standard algorithm)
- Kruskal's MST implementation details (standard algorithm with union-find)
- Whether to expose `default_config()` WASM export for JS-side config initialization
- How to structure the Rust module (one file vs split within each concern)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Support Module Research & Design
- `.planning/HANDOFF.md` — Full architecture decision record, algorithm detail, WASM API spec, risk analysis
- `.planning/research/SUPPORT-GENERATION.md` — Research doc with Unix-style WASM architecture, support algorithm details, integration plan

### Existing Architecture
- `.planning/ROADMAP.md` §Phase 9 — Goal, success criteria, plan list
- `.planning/PROJECT.md` — Project context, key decisions, constraints
- `.planning/REQUIREMENTS.md` — Original requirements (v1/v2/v3)
- `.planning/STATE.md` — Current project state, phase completion history

### Prior Phase Context
- `.planning/phases/05-rust-consolidation/05-CONTEXT.md` — WASM architecture decisions, test strategy, ground-truth pattern
- `.planning/phases/03.5-scoring-expansion/03.5-scoring-expansion-CONTEXT.md` — Scoring expansion patterns, PRNG, ranker decisions

### Codebase Reference
- `crates/orient/src/scoring.rs` — Rust scoring implementations (reuse `perpendicular_basis` if needed)
- `crates/orient/src/stability.rs` — Ray-triangle intersection patterns (Möller-Trumbore reference)
- `crates/geometry-kernel/src/flat.rs` — Repair functions (reference for mesh processing patterns)
- `web/src/pipeline.ts` — Current pipeline orchestration (integration point for support stage)
- `web/src/workers/orient.worker.ts` — Worker pattern to follow for support.worker.ts
- `.opencode/skills/spike-findings-orient-stl/SKILL.md` — WASM rebuild rule, regression verification rule

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/orient/src/scoring.rs:perpendicular_basis()` — tangent plane computation, may be useful for contact point projection
- `crates/orient/src/stability.rs` — Möller-Trumbore ray-triangle intersection pattern (reference for volume.rs ray casting)
- `web/src/workers/orient.worker.ts` — Worker pattern: lazy WASM load, message handling, postMessage result. Copy for support.worker.ts
- `web/src/pipeline.ts` — `runPipeline()` with `onProgress` callback pattern. Add support stage after orient.
- `crates/geometry-kernel/src/flat.rs` — convex hull implementation (gift-wrapping or quickhull) for raft.rs

### Established Patterns
- **Unix-style WASM modules**: each crate is independent, flat `&[f32]` in/out, no cross-crate deps
- **Ground-truth tests**: hand-computed expected values from known geometry (Phase 5 pattern)
- **WASM feature gating**: `#[cfg(feature = "wasm")]` for wasm-bindgen exports, `default = ["wasm"]`
- **serde rename_all**: `#[serde(rename_all = "camelCase")]` for JS interop
- **Cargo.toml dual-target**: `crate-type = ["cdylib", "rlib"]` for both WASM and native test builds

### Integration Points
- `web/src/pipeline.ts` — add `generateSupports?: boolean` to PipelineConfig, `supports?: SupportResult` to PipelineResult
- `Makefile` — add `wasm-support` target, add to `wasm` aggregate target
- `web/package.json` — update `build:wasm` script if needed
- `Cargo.toml` (workspace) — add `support` to `[workspace] members`

</code_context>

<specifics>
## Specific Ideas

- HANDOFF §4.4 circular deformity adjustment: compute local curvature at each contact point via `dot(normal, -direction)`, upgrade density at cylinder bottoms
- HANDOFF §4.3 island detection: 2D per-layer rasterization with connected components (BFS/union-find on grid)
- BVH for ray queries in volume classification — avoid testing all triangles per ray
- Coarse 1mm grid for island detection, refine to 0.5mm for contact placement only
- Poisson-disk with edge seeding: detect grid pixels at island boundary, add extra sample points there
- Raft: convex hull → Delaunay triangulation (Bowyer-Watson) → MST (Kruskal's with union-find) → line connections

</specifics>

<deferred>
## Deferred Ideas

- Tip diameters (0.25/0.40/0.80mm) — deferred to Phase 10 UI as user-preference sliders
- User-editable profile creation UI — future phase
- WebGPU acceleration for support computation — aspirational
- WASM multithreading via SharedArrayBuffer/rayon — not applicable (rayon counterproductive in WASM)

</deferred>

---

*Phase: 09-support-module*
*Context gathered: 2026-08-27*
