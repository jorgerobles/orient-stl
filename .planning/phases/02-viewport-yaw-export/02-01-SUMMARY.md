# 02-01-SUMMARY: three.js Viewport

**Status:** ✅ Complete
**Date:** 2026-07-11 (backfilled)

## Deliverables

| Artifact | Status | Notes |
|---|---|---|
| `web/src/viewport.ts` | ✅ | `Viewport` class — 235 lines |
| `web/src/main.ts` | ✅ | Candidate state, nav wiring, multi-worker orchestration |
| `web/index.html` | ✅ | Viewport container, nav controls, candidate-info span |

## Verification Results

| Check | Result |
|---|---|
| `npx tsc --noEmit` | ✅ passes |
| Model renders after STL load | ✅ verified at localhost:5173 |
| OrbitControls (rotate/pan/zoom) | ✅ works |
| Next/prev cycles candidates | ✅ works |
| Camera auto-frames on load | ✅ `resetCamera()` sizes to bbox |

## What landed beyond plan

- **Build plate**: `GridHelper(60, 20)` + translucent plane at y=0
- **Overhang heatmap**: `colorOverhang()` per-face — red if `n·up < -cos(crit)`, else blue
- **Centroid pivot**: `centerOffset = -centroid` so model rotates around center of mass
- **Lift-to-plate**: `showCandidate` lifts modelGroup by `-bb.min.y` if below grid
- **Live critical-angle slider**: re-colors without recompute

## Open bug (blocks Phase 2 sign-off)

**Candidate poses drift off-center horizontally** — see `todos/pending/bug-candidate-centering.md`.

Root cause (confirmed in code review, `viewport.ts:187-202`): `centerOffset` is computed
once in `loadModel` from the **unrotated** centroid. After `showCandidate` applies a
quaternion, only Y is re-adjusted (lift); X/Z keep the unrotated offset, so rotated
poses drift off-center. Fix: re-derive X/Z centering from the **rotated** bounding box
in `showCandidate` and `applyYaw`.

## Commands

```bash
cd web && npm run dev    # localhost:5173
```
