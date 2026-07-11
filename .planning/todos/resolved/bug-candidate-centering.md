---
title: Bug — candidate poses not centered in viewport above floor
date: 2026-07-11
resolved: 2026-07-11
priority: high
status: fixed-pending-human-visual
context: Reported after testing build at localhost:5173. App works end-to-end but poses drift off-center.
phase: 02
---

**Symptom:** When navigating between ranked candidates, each candidate pose should be
centered in the viewport and resting above the floor grid (y=0). In practice, poses
appear off-center and/or below the build plate for some candidates.

**Expected:** Every candidate pose —
1. Lowest point sits at y=0 (on the grid)
2. Horizontally centered (x/z) in the viewport camera framing
3. Consistent across next/prev navigation

**Suspected root cause:** The lift/centering logic in `web/src/viewport.ts` likely
computes the lift from the *original* bounding box rather than the *rotated* bounding
box. After applying `qFull = qYaw * qAlign(dir, -Y)`, the lowest vertex changes; the
lift amount must be derived from the transformed geometry, not the source mesh.

**Root cause CONFIRMED in code review (2026-07-11):**

`viewport.ts:loadModel` (lines 160-170) computes `centerOffset = -centroid` from the
**unrotated** vertex average and sets `modelGroup.position` to it. This centers the
*original* mesh on the origin.

`viewport.ts:showCandidate` (lines 187-202) and `applyYaw` (lines 204-217) then:
1. Reset `modelGroup.position` to the **unrotated** `centerOffset`
2. Apply the candidate quaternion to `this.mesh.quaternion`
3. Compute `bb = new THREE.Box3().setFromObject(this.mesh)` (rotated bbox)
4. Lift Y by `-bb.min.y` if negative

**The bug:** Step 4 only corrects **Y**. The X/Z drift from rotating around the
unrotated centroid is never corrected. After rotation, the rotated mesh's horizontal
center is no longer at the origin, so the pose appears off-center in the viewport.

**Fix:** In both `showCandidate` and `applyYaw`, after applying the quaternion, re-center
X/Z based on the **rotated** bounding box:

```ts
const bb = new THREE.Box3().setFromObject(this.mesh);
const center = new THREE.Vector3();
bb.getCenter(center);
// X/Z from rotated bbox, Y from lift
this.modelGroup.position.set(
  this.centerOffset.x - center.x,   // re-center horizontally
  this.centerOffset.y - bb.min.y,   // lift onto plate
  this.centerOffset.z - center.z,   // re-center horizontally
);
```

(Note: `centerOffset.y` already shifts the unrotated centroid to origin; the lift
`-bb.min.y` raises the lowest rotated point to y=0.)

**Repro:** Load any STL (e.g. `test-tetrahedron.stl`) at localhost:5173, click through
several candidates, observe off-center / below-grid poses.

**Files to investigate:**
- `web/src/viewport.ts` — lift + centering logic
- `web/src/compute.ts` — `computeSlice` quaternion composition (verify qAlign applied)

**Fix approach (TDD):**
1. Write a unit test that computes the rotated bounding box min-Y for a known
   quaternion and asserts the lift equals `-rotatedMinY`.
2. Apply lift from rotated bbox, re-center camera target on rotated centroid.
