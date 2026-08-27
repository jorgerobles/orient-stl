# Debug Handoff: Phase 10 Support Rendering

**Date**: 2026-08-27
**Phase**: 10-ui-integration
**Status**: 9 supports generated but positions slightly outside mesh bbox; spheres not visible in viewport

---

## Root Cause Analysis

### Fixed bugs (in this session)
1. **Grid coordinate mismatch** — Island pixels were grid cell indices, but placement used `cell * cell_size` without adding grid_origin. FIXED: added `grid_origin` to Island struct.
2. **Ray direction reversed** — `create_contact_point` cast ray in `dir` direction (away from mesh). FIXED: cast in `-dir` (toward mesh).
3. **Base point wrong** — Base was `position - raft_thickness` instead of projecting to raft plane. FIXED: project along build direction.
4. **No mesh decimation** — Full 1.5M triangle mesh was too slow for WASM support. FIXED: decimate to 100K faces.
5. **Double rotation attempted** — Rotated positions that WASM already accounted for via direction vector. FIXED: removed rotation.
6. **Centroid offset missing** — SupportRenderer didn't apply centroidTranslate offset. FIXED: added `setOffset`.
7. **Support worker URL wrong** — `./workers/` vs `../workers/`. FIXED.

### Remaining bug: Coordinate system mismatch in ray origin
The island detection projects vertices onto the build direction to get heights:
```rust
height = -(x * dir[0] + y * dir[1] + z * dir[2]);
```

The 2D grid uses raw mesh X,Y coordinates. Heights are projected values. But `create_contact_point` constructs a ray origin mixing these two spaces:
```rust
let ray_origin = [point_2d[0], point_2d[1], island.z_max + 1.0];
//                        ^^^mesh X        ^^^mesh Y     ^^^projected height (NOT mesh Z!)
```

This means the ray originates from a point that doesn't exist in 3D mesh space. The ray-triangle intersection then finds triangles near this phantom point, producing contact positions that are slightly outside the actual mesh bounding box.

**Evidence**: Mesh bbox Z range is [-10.62, 22.03], but contact positions have Z = 21-31 (after centroid offset).

### The fix needed
The ray origin needs to be a real 3D point above the mesh in mesh coordinate space. Two approaches:

**Option A**: Cast the ray from a point high above the mesh along `-dir`:
```rust
// Instead of using island.z_max as Z, compute a point well above the mesh
let mesh_center_z = (island.z_min + island.z_max) / 2.0; // still projected height
// Need to un-project this back to mesh space, or just use a large offset
```

**Option B** (simpler): Instead of 2D grid → 3D ray, use the actual 3D mesh surface directly. Find triangles whose centroids are near the island's projected position, then use those triangle positions directly for contact placement. This avoids the coordinate system mismatch entirely.

---

## Current Architecture

### Support generation flow (working)
```
User clicks "Generate Supports"
  → AppController.runSupportGeneration(quaternion)
    → decimateForScore(lod, 100_000) — reduce to 100K faces
    → Worker postMessage({ positions, normals, areas, direction, config })
      → support.worker.ts loads WASM
      → wasm.generate_supports(positions, normals, areas, direction, config)
        → island::detect_islands() — 2D rasterization + connected components → 27 islands
        → volume::compute_volume_above() — ray cast for volume classification
        → placement::place_contacts() — Poisson-disk sampling → 9 contacts
        → raft::generate_raft() — convex hull + MST
      → returns SupportResult
    → AppController stores in state
    → SupportRenderer.render() — creates cylinders + debug spheres
    → viewport.setSupportVisible(true)
```

### Data flow (coordinate spaces)
1. **Pipeline positions**: Convention-transformed (Y-up/Z-up), centered at origin via centroidTranslate
2. **WASM input**: Decimated positions in pipeline space
3. **Island detection**: Projects onto build direction, rasterizes in XY plane
4. **Contact placement**: Returns positions in pipeline space (with coordinate bug)
5. **SupportRenderer**: Applies centroidTranslate offset → modelGroup local space
6. **Viewport**: modelGroup positioned at y=boundingRadius, mesh rotated by quaternion

### Key coordinates (from last Puppeteer run)
- Mesh bbox (modelGroup local): [-12.35, -11.89, -10.62] → [12.53, 8.74, 22.03]
- Centroid offset: [-2.38, 0.18, 24.63]
- Contact position (raw WASM): Z ≈ 1.76 (within mesh Z range -35.25 to -2.6 in raw space)
- Contact position (after offset): Z ≈ 26.39 (OUTSIDE mesh local Z range -10.62 to 22.03)

---

## Key Files

| File | Status |
|------|--------|
| `crates/support/src/placement.rs` | Ray direction fixed, grid_origin used, base projection fixed. Coordinate bug remains. |
| `crates/support/src/island.rs` | grid_origin stored in Island. Algorithm only works for Z-aligned directions. |
| `crates/support/src/types.rs` | Island struct has grid_origin field. |
| `crates/support/src/lib.rs` | Computes mesh height range for raft_height. Passes to place_contacts. |
| `crates/support/src/volume.rs` | Uses world-space centroid for volume computation. |
| `web/src/app/AppController.ts` | runSupportGeneration decimates mesh, calls support worker, renders results. Has debug logging. |
| `web/src/viewport/SupportRenderer.ts` | Renders columns + debug spheres. Has centroid offset. |
| `web/src/viewport/Viewport.ts` | Passes centroid offset to SupportRenderer. |
| `web/src/workers/support.worker.ts` | Loads WASM, calls generate_supports. |
| `web/test-supports.mjs` | Puppeteer test script for headless verification. |

---

## Test Commands
```bash
# Rust tests
cargo test -p support

# WASM rebuild
wasm-pack build crates/support --target bundler --out-dir ../../web/pkg/support

# TypeScript check
cd web && npx tsc --noEmit

# Web tests
cd web && npx vitest run

# Puppeteer visual test
cd web && node test-supports.mjs
```

---

## Git Log (recent)
```
ef237d3 fix(support): ray direction, raft base projection, decimation, position alignment
50e0843 fix(support): grid coordinate mismatch causing 0 contacts
036df08 refactor(10-01): replace support toggle with Generate/Remove/Export buttons
00ed798 feat(10-03): STL export with support geometry merge
d3c748e feat(10-02): support geometry rendering in viewport
aaca447 feat(10-01): add support toggle + config panel UI
```

---

## What the User Wants
1. Supports that are visually connected to the mesh surface (not floating in space)
2. Support columns from build plate to overhang surface
3. Enough contacts to be useful (currently 9 from 27 islands)
4. Generate / Remove / Export buttons that work
5. Visual quality similar to Lychee (tapered columns, colored by type)
