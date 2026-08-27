# Debug Handoff: Phase 10 Support Rendering

**Date**: 2026-08-27
**Phase**: 10-ui-integration
**Status**: Supports generate (0 contacts, 25 islands) but don't render visibly

---

## Current State

### What works
- `make wasm` builds all 5 WASM binaries including `support_bg.wasm`
- UI loads, scores orientations, navigates candidates — all Phase 1-9 features intact
- SupportPanel renders with Generate Supports / Remove / Export buttons
- `generate_supports` WASM function is callable from support worker
- Island detection works (25 islands found on test mesh)
- TypeScript compiles, 78 tests pass

### What's broken
1. **0 contacts from support placement** — WASM finds 25 islands but places 0 contact points
2. **Support rendering connects to nothing** — stray cylinders not anchored to mesh surface
3. **Recalculate button disabled after file load** — `markClean()` disables it, only `markDirty()` re-enables
4. **User wants Generate button to work on-demand** — currently calls `spawnCompute` (scoring) then `runSupportGeneration`, but the scoring step may be redundant

---

## Architecture Summary

### Support generation flow
```
User clicks "Generate Supports"
  → AppController.runSupportGeneration(quaternion)
    → Creates Worker from '../workers/support.worker.ts'
    → Posts: { type: 'support', positions, normals, areas, direction, config }
    → Support worker loads '../../pkg/support/support.js' WASM
    → Calls wasm.generate_supports(positions, normals, areas, direction, config)
    → Returns SupportResult { supports: Support[], raft: RaftGeometry, totalVolume, islandCount }
  → AppController stores in state, calls viewport.renderSupports()
  → SupportRenderer creates THREE.Group children in modelGroup
```

### Support WASM input
```rust
pub fn generate_supports(
    positions: &[f32],   // flat: [x,y,z, x,y,z, ...] per triangle vertex
    normals: &[f32],     // flat: [nx,ny,nz, ...] per triangle
    areas: &[f32],       // flat: [area, ...] per triangle
    direction: &[f32],   // [dx, dy, dz] build direction (up vector)
    config: JsValue,     // SupportConfig JSON
) -> JsValue;            // SupportResult JSON
```

### Key data flow issue
- `lod.positions` passed to support worker is the **full un-decimated** mesh from `lastOriData`
- The viewport renders the same positions (centered via `centroidTranslate`, positioned at `boundingRadius`)
- SupportRenderer adds columns to `modelGroup`, so they should rotate with the model
- But the WASM contact placement may produce 0 contacts if thresholds/scale don't match

---

## Issues to Debug (priority order)

### Issue 1: 0 contacts despite 25 islands
**Where**: `support/src/lib.rs` — the `generate_supports` WASM function
**Symptom**: Island detection finds 25 islands, but contact placement returns 0 contacts
**Likely cause**: 
- Poisson-disk spacing thresholds in config vs actual mesh scale
- Contact placement algorithm failing silently
- Input data format mismatch (positions may need specific scale/normalization)

**Debug steps**:
1. Read `support/src/lib.rs` — find `generate_supports`, trace island→contact flow
2. Check `support/src/placement.rs` — Poisson-disk sampling, what makes it return 0 points
3. Check `support/src/island.rs` — what does the island struct contain? Are island surfaces valid?
4. Log the actual config values being passed (thresholds, spacing)
5. Check if positions are in mm scale (STL default) or some other unit

### Issue 2: Support columns not connecting to mesh
**Where**: `web/src/viewport/SupportRenderer.ts` — `renderColumn()`
**Symptom**: Cylinders float in space, don't attach to model surface
**Likely cause**:
- Contact `base` and `position` coordinates are in mesh-local space but model is centered/translated
- The `centroidTranslate` offset in `loadModel` isn't applied to support positions
- Support group is in `modelGroup` but columns use raw WASM coordinates

**Debug steps**:
1. Check `Viewport.loadModel()` — what transform does it apply to the mesh?
2. Check if `contact.base` and `contact.position` from WASM are in the same coordinate space
3. The model is centered at origin and translated to `y=boundingRadius` — supports need the same centering

### Issue 3: Recalculate button stays disabled
**Where**: `web/src/app/AppController.ts` — `markClean()` / `markDirty()`
**Symptom**: Button enabled only after config change, not after file load
**Likely cause**: `markClean()` calls `enableRecalc(false)` — by design, but user expects it to be available

---

## Key Files

| File | Role |
|------|------|
| `web/src/app/AppController.ts` | Main controller — `runSupportGeneration()` at line ~509 |
| `web/src/workers/support.worker.ts` | Support worker — loads WASM, calls `generate_supports` |
| `web/src/viewport/SupportRenderer.ts` | Renders support columns + raft in three.js |
| `web/src/viewport/Viewport.ts` | Scene management, `renderSupports()` / `clearSupports()` |
| `web/src/views/SupportPanel.ts` | Generate/Remove/Export buttons UI |
| `web/src/pipeline.ts` | Full pipeline (Stage 5 = support generation) |
| `web/src/loadSTL.ts` | `loadWithProgress()` — calls pipeline |
| `web/src/types.ts` | `SupportConfig`, `SupportResult`, `ContactPoint`, `Support`, `RaftGeometry` |
| `support/src/lib.rs` | WASM entry point — `generate_supports` |
| `support/src/island.rs` | Island detection (2D slice rasterization) |
| `support/src/placement.rs` | Contact point placement (Poisson-disk) |
| `support/src/volume.rs` | Volume classification |
| `support/src/raft.rs` | Raft generation |
| `web/pkg/support/support_bg.wasm` | Built WASM binary |

---

## Git Log (recent Phase 10 commits)

```
036df08 refactor(10-01): replace support toggle with Generate/Remove/Export buttons
00ed798 feat(10-03): STL export with support geometry merge
d3c748e feat(10-02): support geometry rendering in viewport
aaca447 feat(10-01): add support toggle + config panel UI
```

---

## What the User Wants

1. **Generate Supports button** that produces visible support columns attached to the model
2. **Remove button** that clears supports
3. **Export with Supports** button that merges support geometry into STL
4. **Recalculate** should work (regenerate supports after re-orienting)
5. Supports should look like Lychee: tapered columns (wider base, narrower tip), colored by type, connected to mesh surface

---

## Constraints

- WASM rebuild required after any Rust change: `wasm-pack build support --target bundler --out-dir web/pkg`
- TypeScript: `cd web && npx tsc --noEmit`
- Tests: `cd web && npx vitest run`
- 78 tests currently pass
