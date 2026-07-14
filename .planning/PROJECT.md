# orient-stl

## What This Is

A browser-based tool for resin 3D printing that automatically finds optimal print orientations for STL models. It calculates overhang penalties across candidate orientations, ranks them, lets the user navigate the ranking in a single three.js viewport, adjust yaw with snap-to-geometry, and export oriented STLs individually or as a ZIP bundle.

## Core Value

Generate a reliable orientation ranking that minimizes supports and maximizes print success, without the user manually rotating the model.

## Requirements

### Validated

(None yet — ship to validate)

### Validated

- [x] STL file loading & parsing (binary STL, via stl-io in Rust WASM)
- [x] Overhang penalty scoring (area-weighted, S² space)
- [x] Candidate generation from convex hull normals (hull mode)
- [x] Binary stability reject (CoM projection inside contact footprint)
- [x] Ranked results display with next/prev navigation
- [x] Yaw adjustment with slider + snap-to-geometry
- [x] WASM core computation (Rust → wasm-bindgen)
- [x] three.js single viewport for orientation preview
- [x] Height-weighted scoring (v2)
- [x] hull_plus_sphere candidate sampling (v2)
- [x] S² hill-climbing refinement (v2)
- [x] All metrics & ranking in Rust (no TS duplication)
- [x] CLI binary for headless verification

### Out of Scope

- Slicer functionality (gcode generation, support structure generation) — use existing slicers
- Network/multiplayer features — local tool only
- Native desktop app — browser-first with WASM core
- ASCII STL support — deferred unless needed (binary covers all common cases)
- Thumbnail strip (OffscreenCanvas rendering) — YAGNI, single-candidate viewport sufficient
- Favorites/IndexedDB persistence — YAGNI, single-session workflow covers use case
- Multi-STL ZIP export — YAGNI, single-file export covers essential workflow

## Context

This tool addresses a gap in the resin printing workflow: existing slicers (PrusaSlicer, Lychee) offer auto-orientation but don't let the user browse a ranked list of candidates and pick the best tradeoff. The implementation uses a Rust WASM core for scoring with the insight that the orientation score depends only on the `down_local` direction (2 DOF on S²), not full rotation — enabling efficient candidate evaluation without rotating the mesh.

## Constraints

- **Tech stack**: Rust → WASM for computation, JS/TS + three.js + Vite for UI
- **Target**: Browser (wasm32-unknown-unknown), no Node.js server needed
- **STL parsing**: Binary only (stl-io crate, zero deps); ASCII deferred
- **Convex hull**: Vendored quickhull in Rust, no qhull or ndarray crates
- **Persistence**: IndexedDB for blobs (thumbnails), never localStorage
- **Export**: Single STL or ZIP via fflate (client-side only)

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Score depends only on down_local (S², not SO(3)) | Yaw is invariant for overhang — reduces search space from 3 to 2 DOF | ✓ Good |
| Vendored quickhull, no crate | Smaller WASM binary, no BLAS/rayon issues on wasm target | ✓ Good |
| stl-io for STL parsing | Zero dependencies, takes impl Read, works with Cursor<&[u8]> in WASM | ✓ Confirmed (spike resolved) |
| WASM boundary is `prepare_data()`, not `compute_orientations()` | Pragmatic split: WASM does one-time geometry prep, JS workers do per-candidate scoring for iteration speed | ✓ Good — defer move-back-to-WASM until JS proves slow |
| Phase 2 scope (viewport/export/heatmap) landed in Phase 1 | Building the viewport was the natural way to *see* candidates; DOM-only interim skipped | ✓ Good — Phase 2 tracking must reconcile |
| Centroid (vertex average) as rotation pivot | Center of mass for uniform density; stable rotation | ✓ Good |
| Full quaternion = `qYaw * qAlign(dir, -Y)` | Align candidate dir to -Y first, then apply yaw — fixes below-build-plate rendering | ✓ Good (verify per candidate) |
| Multi-worker: split directions across `hardwareConcurrency - 1` | Parallel scoring ~4x on quad-core, no cross-worker sync | ✓ Good |
| Decimate to ~12K elements for scoring | 500K×400 = 200M iters → 12K = 50x faster, rank order preserved | ✓ Good |
| WASM-first for computation; WebGPU is upgrade path | Don't add JS SIMD/WASM threads/Node deps; WebGPU for future GPU-class throughput | — Aspirational (v2/v3) |
| Binary stability reject in v1 | Cheap geometric predicate (~40 lines), prevents ranking recommending unstable orientations | — Pending |
| Yaw control via circular dial + snap | Rotating calipers reused from bbox computation; snap from hull edge aligns | — Pending |
| Coarse granularity (4 phases) | Aligns with spec's existing v1/v2/v3 roadmap | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-07-11 — GSD state reconciled with codebase after Phase 1 completion and Phase 2 implementation drift*
