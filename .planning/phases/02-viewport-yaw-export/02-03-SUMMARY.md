# 02-03-SUMMARY: STL Export

**Status:** ✅ Complete
**Date:** 2026-07-11 (backfilled)

## Deliverables

| Artifact | Status | Notes |
|---|---|---|
| `web/src/exportSTL.ts` | ✅ | Binary STL writer, 55 lines |
| `web/src/main.ts` export wiring | ✅ | Quaternion compose + rotate + download |

## Verification Results

| Check | Result |
|---|---|
| Binary STL format correct | ✅ 80-byte header + uint32 count + 50B/triangle |
| Per-triangle normal recomputed | ✅ from cross product, normalized |
| Full quaternion applied | ✅ `qYaw * candidateQuat` in main.ts:280 |
| Blob download triggers | ✅ `{baseName}_orient_{N}.stl` |
| Export disabled until candidates ready | ✅ `display:none` until compute done |

## Implementation notes

- `exportSTL(positions, name, candidateIndex)` takes already-rotated positions
- `rotatePositions` (main.ts:293) applies quaternion via the standard rotation formula
- `multiplyQuats` (main.ts:284) is a local Hamilton product — duplicates compute.ts
  version because main.ts doesn't import it (minor cleanup opportunity)
- Export uses the **full** positions array (not decimated) — decimation is scoring-only

## Open verification (human)

The handoff noted export needs final human verification that the downloaded STL
matches the viewport orientation. Code path looks correct (same quaternion math as
viewport), but a side-by-side check is still pending.

## Commands

Exercise via dev server: load STL → navigate candidate → adjust yaw → click Export STL.
