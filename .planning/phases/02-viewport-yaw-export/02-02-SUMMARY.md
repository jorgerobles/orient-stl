# 02-02-SUMMARY: Yaw Control

**Status:** ⚠️ Partial — 2 of 5 must-haves missing
**Date:** 2026-07-11 (backfilled)

## Deliverables

| Artifact | Status | Notes |
|---|---|---|
| `web/src/compute.ts::computeDefaultYaw` | ✅ | Rotating-calipers bbox-minimizing yaw (used as base) |
| `web/src/viewport.ts::applyYaw` | ✅ | Premultiplies yawQ onto baseQ, re-colors, re-lifts |
| `web/index.html` yaw-panel | ✅ | Slider + value + snap + reset buttons |
| Circular dial | ❌ | Linear slider instead |
| Geometry snap | ❌ | Fixed 45° increments instead |

## Verification Results

| Check | Result |
|---|---|
| Slider adjusts yaw live | ✅ works |
| Yaw applies to viewport | ✅ model rotates around Y |
| Reset restores auto yaw | ✅ restores `candidates[i].quaternion` |
| Yaw included in export | ✅ `qYaw * candidateQuat` in main.ts:280 |
| **Circular dial drag** | ❌ not implemented |
| **Snap to hull-edge angles** | ❌ snaps to fixed 45° |

## Deviations from Plan

The yaw control was simplified to a linear slider + fixed-45° snap button during
rapid prototyping. The planned circular dial with magnetic geometry-snap was deferred.
The underlying math (`computeDefaultYaw` via rotating calipers) is implemented and used
as the base orientation, so the snap feature is a UI-layer gap, not an algorithm gap.

**Impact on requirements:** YAW-01 (adjust yaw) ✅, YAW-02 (numeric) ⚠️ partial,
YAW-03 (snap to geometry) ❌, YAW-04 (reset) ✅, YAW-05 (dial) ❌.

## Deferred to Phase 2 polish

- Circular dial component (SVG arc drag)
- Hull-edge snap angles from rotating calipers output
- Numeric type-in field

## Commands

No standalone command — exercised via dev server at localhost:5173.
