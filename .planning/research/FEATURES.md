# Feature Landscape

**Domain:** Resin 3D printing auto-orientation tool
**Researched:** 2026-07-11

## Table Stakes

Features users expect. Missing = product feels incomplete.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| STL file loading | Every STL tool must accept STL files | Low | Binary STL only in v1. Drag-drop or file picker. |
| 3D viewport with orbit controls | Users need to inspect the model | Medium | three.js with OrbitControls. Must show model at current candidate orientation. |
| Candidate orientation display | The whole point of the tool | High | Next/prev navigation, quaternion interpolation. |
| Export oriented STL | Must get result out | Medium | Apply quaternion to vertices, write binary STL. |
| Yaw adjustment | Users need to control rotation around vertical axis | Medium | Circular dial (not linear slider). Snap-to-geometry at 0/90/180/270 + bbox minima. |

## Differentiators

Features that set product apart. Not expected, but valued.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Ranked candidate list | Existing slicers (PrusaSlicer, Lychee) offer auto-orient but don't show ranked alternatives | High | This is the core differentiator. Let user browse ~50 candidates sorted by score. |
| Multi-metric ranking | Show overhang, height, stability separately — let user sort by any metric | Medium | Unlike a single composite score, exposes tradeoffs. |
| Overhang penalty visualization | Heatmap or highlight of problematic areas per candidate | Medium | Color faces by cos_i value. Helps user understand WHY a candidate scored well/poorly. |
| Stability check | Automatically reject/reduce-rank orientations that would fall over | Medium | Binary reject in v1, continuous margin in v2. Critical for resin printing success. |
| Yaw snap to geometry | Snap to bounding-box minima and edge alignments | Medium | Reuses rotating calipers from hull computation. |
| Multi-file ZIP export | Batch export several candidate orientations at once | Low | fflate makes this trivial once single STL export works. |

## Anti-Features

Features to explicitly NOT build.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| Full slicer (gcode generation) | Enormous scope, existing tools do it well | orient-stl is a pre-slicer step. Output oriented STLs, import into slicer. |
| Support generation | Complex geometry problem, existing slicers have tuned implementations | Focus on minimizing supports via orientation, not generating them. |
| Mesh repair / decimation | Not the tool's purpose | Assume clean manifold STLs. Defer to mesh repair tools. |
| Network / cloud features | Local-only tool. User privacy, no server cost | All computation in browser. No data leaves the machine. |
| Real-time orientation drag | Rotating model and computing score in real-time is expensive for large meshes | Pre-compute ranked list, then navigate with instant quaternion flips. |
| Multi-model layout (packing) | Separate problem (nesting/packing on build plate) | Export individual oriented STLs; use a dedicated packing tool. |

## Feature Dependencies

```
STL file load
  └── STL binary parser (stl.rs)
        └── Mesh precomputation (mesh.rs)
              ├── Convex hull (hull.rs)
              │     └── Candidate generation (candidates.rs)
              │           ├── Scoring (scoring.rs)
              │           └── Stability checking (stability.rs)
              │                 └── Refinement (refine.rs)
              │                       └── compute_orientations() API
              │                             └── three.js viewport (viewport.ts)
              │                                   └── Yaw dial (yaw-dial.ts)
              └── STL export (needs original geometry + candidate quaternion)
                    └── Single STL export
                          └── ZIP multi-export (exportSTL.ts)

OffscreenCanvas thumbnail generation (thumbnails.ts)
  └── Only needs candidate quaternions + geometry — independent of stability/refinement

IndexedDB favorites (favorites.ts)
  └── Depends on thumbnail generation (stores PNG blobs)
```

## MVP Recommendation

Prioritize:
1. **STL file loading + WASM compute pipeline** — Parser, mesh precompute, hull, candidate generation, scoring, stability, single API
2. **three.js viewport with next/prev navigation** — Display model at each candidate orientation
3. **Single STL export** — Apply quaternion, download
4. **Yaw adjustment dial** — After user picks a candidate, fine-tune yaw

Defer:
- Thumbnails (v3): Can navigate by candidate index without thumbnails
- Favorites/IndexedDB (v3): Core UX works without persistence
- ZIP export (v3): Single STL export covers 80% of use cases
- Multi-metric sorting (v2): v1 can sort by composite score only
- Height-weighted scoring (v2): area-weighted only in v1
- Refinement/hill-climbing (v2): Hull candidates are good enough for v1
- hull_plus_sphere mode (v2): Hull mode alone covers most models

## Sources

- orient-spec.md — Project specification defining feature set
- Competitor analysis: PrusaSlicer auto-orient, Lychee Slicer auto-orient, AutoOrientation (Unity)
- Community resin printing knowledge: overhang angles, stability requirements, common workflow patterns
