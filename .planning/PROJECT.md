# orient-stl

## What This Is

A browser-based tool for resin 3D printing that automatically finds optimal print orientations for STL models. It calculates overhang penalties across candidate orientations, ranks them, lets the user navigate the ranking in a single three.js viewport, adjust yaw with snap-to-geometry, and export oriented STLs individually or as a ZIP bundle.

## Core Value

Generate a reliable orientation ranking that minimizes supports and maximizes print success, without the user manually rotating the model.

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] STL file loading & parsing (binary STL, via stl-io in Rust WASM)
- [ ] Overhang penalty scoring (area-weighted, S² space)
- [ ] Candidate generation from convex hull normals (hull mode)
- [ ] Binary stability reject (CoM projection inside contact footprint)
- [ ] Ranked results display with next/prev navigation
- [ ] Yaw adjustment with circular dial + snap-to-geometry
- [ ] WASM core computation (Rust → wasm-bindgen)
- [ ] three.js single viewport for orientation preview
- [ ] Height-weighted scoring (v2)
- [ ] hull_plus_sphere candidate sampling (v2)
- [ ] S² hill-climbing refinement (v2)
- [ ] Offscreen thumbnail generation for each candidate (v3)
- [ ] IndexedDB persistence for favorites (v3)
- [ ] Multi-STL export via fflate ZIP (v3)

### Out of Scope

- Slicer functionality (gcode generation, support structure generation) — use existing slicers
- Network/multiplayer features — local tool only
- Native desktop app — browser-first with WASM core
- ASCII STL support — deferred unless needed (binary covers all common cases)

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
| stl-io for STL parsing | Zero dependencies, takes impl Read, works with Cursor<&[u8]> in WASM | ✓ Good |
| Binary stability reject in v1 | Cheap geometric predicate (~40 lines), prevents ranking recommending unstable orientations | — Pending |
| Yaw control via circular dial + snap | Rotating calipers reused from bbox computation; snap from hull edge aligns | — Pending |
| Coarse granularity (3 phases) | Aligns with spec's existing v1/v2/v3 roadmap | — Pending |

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
*Last updated: 2026-07-11 after project initialization*
