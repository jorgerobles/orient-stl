---
status: resolved
trigger: "each candidate pose should be centered in viewport above the floor (user-reported after testing at localhost:5173)"
slug: candidate-pose-centering
created: 2026-07-11
updated: 2026-07-11
phase: 02
---

# Debug Session: candidate-pose-centering

## Symptoms

**Expected behavior:**
Each candidate pose, when navigated to via next/prev, should be:
1. Resting on the build plate grid (lowest point at y=0)
2. Horizontally centered (x/z) in the viewport camera framing
3. Consistent across all candidates

**Actual behavior:**
Poses appear off-center and/or below the build plate for some candidates. The model
drifts horizontally when navigating between candidates.

**Error messages:**
None — pure visual bug, no console errors.

**Timeline:**
Present since the quaternion composition fix (`qYaw * qAlign(dir, -Y)`) was added to
`computeSlice`. Reported by user after testing the live build at localhost:5173.

**Reproduction:**
1. Start dev server (`cd web && npm run dev`)
2. Load any STL (e.g. `test-tetrahedron.stl`) at http://localhost:5173/
3. Click "Next" through several candidates
4. Observe: poses drift off-center horizontally; some sit below the grid

## Root Cause Hypothesis (pre-analyzed, needs verification)

`web/src/viewport.ts:loadModel` (lines 160-170) computes `centerOffset = -centroid` from
the **unrotated** vertex average and sets `modelGroup.position` to it.

`showCandidate` (lines 187-202) and `applyYaw` (lines 204-217) then:
1. Reset `modelGroup.position` to the **unrotated** `centerOffset`
2. Apply the candidate quaternion to `this.mesh.quaternion`
3. Compute rotated bbox via `Box3().setFromObject(this.mesh)`
4. Lift Y by `-bb.min.y` if negative

**Only Y is re-adjusted after rotation.** The X/Z drift from rotating around the
unrotated centroid is never corrected, so rotated poses appear off-center.

## Current Focus

- **hypothesis:** centerOffset uses the unrotated centroid; only Y is corrected after rotation, leaving X/Z off-center — CONFIRMED
- **test:** web/src/centering.test.ts (4 tests against pure `recenterPosition`)
- **expecting:** the test fails with current code (x/z center drifts) — CONFIRMED RED
- **next_action:** DONE — root cause verified by failing test, fix applied, tests green, type-check clean
- **reasoning_checkpoint:** resolved
- **tdd_checkpoint:** green

## Evidence

- timestamp: 2026-07-11 — Extracted centering math into pure `recenterPosition(centerOffset, bboxMin, bboxCenter)` in web/src/centering.ts (DOM/three.js-free, unit-testable).
- timestamp: 2026-07-11 — Wrote web/src/centering.test.ts with a concrete rotated-bbox scenario (asymmetric tetrahedron-like vertices, 90° Y rotation). RED confirmed: `centers the rotated bbox at x=0 and z=0` failed with `expected -2.5 to be close to -5`, proving the X/Z drift bug is reproducible at the math level.
- timestamp: 2026-07-11 — Applied minimum fix: `x = centerOffset.x - bboxCenter.x`, `z = centerOffset.z - bboxCenter.z` (Y behavior preserved — lift only when below plate). GREEN: 4/4 tests pass.
- timestamp: 2026-07-11 — Refactored viewport.ts showCandidate + applyYaw to call recenterPosition using the ROTATED world bbox (Box3.setFromObject + getCenter).
- timestamp: 2026-07-11 — `npx tsc --noEmit` exit 0 (0 type errors); `npx vitest run` 4/4 passing.

## Eliminated

- Y-lift logic itself is correct (only-lift-when-negative behavior preserved; not the cause of horizontal drift).

## Resolution (REVISED — first fix was wrong, this is the real fix)

- **root_cause (revised):** The mesh rotated around its geometry's local (0,0,0), which is an ARBITRARY CORNER of the model — NOT the centroid. The first fix (post-rotation X/Z recentering via `recenterPosition`) only moved the rotated AABB back; the mesh still ORBITED because the rotation pivot was wrong. The user correctly identified: "no estás usando el centro local de la malla, sino la del mundo o con otro pivote."
- **fix (correct):** Bake the centroid-centering INTO the geometry via `geometry.translate(-cx,-cy,-cz)` at load time. Now the mesh's local origin IS its centroid, so `mesh.quaternion` rotates in-place (spin, not orbit). `modelGroup` stays at world X=Z=0; only Y is lifted (`liftOntoPlate(minY)`) to rest on the plate. Removed the broken `centerOffset` group-offset hack and `recenterPosition`.
- **verification:** `npx tsc --noEmit` exit 0; `npx vitest run` 6/6 passing (centroid-bake invariant + Y-lift). Dev server at localhost:5173 ready for human visual confirmation.
- **files_changed:** web/src/centering.ts (rewritten: `centroidTranslate` + `liftOntoPlate`), web/src/centering.test.ts (rewritten: 6 tests on the bake invariant), web/src/viewport.ts (loadModel bakes centroid into geometry; showCandidate/applyYaw lift Y only), web/vitest.config.ts, web/package.json.
