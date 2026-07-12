---
status: resolved
trigger: "missing third rotation axis (p t y) and none of them are working"
created: 2026-07-12
updated: 2026-07-12
---

## Symptoms

- **Expected behavior**: Each ring rotates model in its axis (X, Y, Z, camera)
- **Actual behavior**: No rings work at all — dragging has no effect
- **Error messages**: No console errors
- **Timeline**: Never worked
- **Reproduction**: Load STL → click candidate → drag ring
- **Follow-up**: "not all axes do their part. also need a highlight on hover. the camera ring does not rotate properly from the current view"

## Current Focus

- **hypothesis**: `getNDC()` in viewport.ts doesn't account for canvas DOM offset → raycasting misses rings entirely
- **test**: verified via code review — fix removes all blockers
- **expecting**: fixing NDC calculation allows ring hit detection → axes rotate correctly
- **next_action**: done — all fixes applied

## Evidence

- **code review**: `viewport.ts` line 309-315 — `getNDC()` computes NDC using `clientX / el.clientWidth`, but `clientX`/`clientY` from PointerEvent are viewport-relative coordinates, not canvas-relative. The canvas is inside `.main` which is `grid-column: 2` in a `grid-template-columns: 320px 1fr` layout → canvas has a ~320px x-offset from viewport left edge.

  This causes all raycaster intersection tests (in `raycastAllRings` and `intersectRingPlane`) to compute wrong NDC coordinates, so the ray misses the ring geometry entirely. OrbitControls works correctly because it uses `getBoundingClientRect()` internally — our custom code did not.

- **code review**: `viewport.ts` line 251 — `gizmoRingX` rotation was `rotation.z = Math.PI / 2`. Three.js `TorusGeometry` defaults to XY plane (hole along Z). Rotating around Z keeps the torus in the XY plane — same orientation as the Z ring. The X ring should use `rotation.y = Math.PI / 2` to orient in the YZ plane (rotation around X axis). This is a secondary bug affecting only the X ring's visual/drag alignment.

- **Axis rotation verification**: After fixing getNDC and X ring rotation, all three axes are correctly wired:
  - **X ring** (red): ring in YZ plane, `dragAxisVec = (1,0,0)`, angle measured around X axis ✓
  - **Y ring** (green): ring in XZ plane (`rotation.x = PI/2`), `dragAxisVec = (0,1,0)`, angle measured around Y axis ✓
  - **Z ring** (blue): ring in XY plane, `dragAxisVec = (0,0,1)`, angle measured around Z axis ✓
  - **Camera ring** (white): uses `cameraUp` and `cameraRight` derived from current camera quaternion, applies `qx(cameraUp, dx) * qy(cameraRight, dy) * startQuat`. This correctly rotates the model relative to the current view axes. If the camera ring feels off, it may be a sensitivity (0.005 factor) or lack of visual reference (billboarded ring has no orientation cues) rather than a math bug.

## Eliminated

- `intersectObjects` recursive flag — meshes passed directly to array, recursion irrelevant
- OrbitControls consuming events — our handlers use `capture: true` and call `stopPropagation()`
- Ring geometry not rendering — gizmo is visible (model renders, rings are children of gizmoGroup)
- `boundingRadius` calculation — verified correct in `centering.ts`
- Pointer events not reaching canvas — no CSS `pointer-events: none` on viewport/canvas
- `angleAroundAxis` math — verified: projects onto perpendicular plane, computes signed angle via `atan2(axis · (proj × ref), proj · ref)` ✓
- Camera ring rotation math — uses camera's current `quaternion` and `getWorldDirection` to compute view-relative axes. OrbitControls disabled during drag, so axes stay constant per drag session.

## Changes Applied

1. **`getNDC()` fix** (was PRIMARY blocker): Now uses `getBoundingClientRect()` to compute NDC from canvas-relative coordinates instead of viewport-raw clientX/clientY.
2. **`gizmoRingX.rotation` fix**: Changed from `rotation.z = PI/2` to `rotation.y = PI/2` so the X ring sits in the YZ plane (correct for rotation around X axis).
3. **Hover highlighting** (feature addition): Rings now highlight to full opacity (1.0) when the pointer hovers over them. Original opacity is restored when pointer leaves. Hover detection runs in the `pointermove` handler before drag mode activates.
4. **`destroyGizmo` cleanup**: Clears `ringDefaultOpacities` map and resets `hoveredRing` on gizmo destruction.

## Resolution

- **root_cause**: `getNDC()` in `viewport.ts` doesn't adjust for canvas element's viewport offset (needs `getBoundingClientRect()`), causing all ring raycasts to miss their targets
- **fix**: applied — replaced `getNDC` calculation to use `getBoundingClientRect()`; fixed `gizmoRingX` rotation axis; added hover highlighting; swapped Y/Z ring drag axes to match torus visual orientation; unified camera ring to rotate around camera view direction axis (roll) using angle-tracking machinery
- **verification**: `npm run type-check` passes (0 errors); user confirmed all 4 rings working correctly
- **files_changed**: web/src/viewport.ts
- **fix_type**: NDC coordinate calculation + ring rotation axis + hover highlighting + axis swap + camera ring roll
