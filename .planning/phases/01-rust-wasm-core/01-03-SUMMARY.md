# 01-03-SUMMARY: JS Bridge & File Input

**Status:** ✅ Complete
**Date:** 2026-07-11

## Deliverables

| Artifact | Status | Notes |
|---|---|---|
| `web/src/types.ts` | ✅ | `OrientConfig`, `defaultConfig()` |
| `web/src/loadSTL.ts` | ✅ | `initWasm()`, `loadSTLBytes()`, `prepareData()` bridge |
| `web/src/main.ts` | ✅ | WASM init, file picker + drag-drop, result display |
| `web/index.html` | ✅ | Drop zone, file picker, config panel, results |
| `web/src/compute.ts` | ✅ | (Beyond plan) full scoring/stability/yaw/height/slice pipeline in JS |
| `web/src/orient.worker.ts` | ✅ | (Beyond plan) Web Worker entry for parallel candidate scoring |
| `web/src/viewport.ts` | ✅ | (Beyond plan) three.js viewport — see Phase 2 |
| `web/src/exportSTL.ts` | ✅ | (Beyond plan) binary STL writer — see Phase 2 |

## Verification Results

| Check | Result |
|---|---|
| `npx tsc --noEmit` | ✅ passes |
| `npx vite build` | ✅ builds |
| Dev server at localhost:5173 | ✅ serves app |
| End-to-end: file → WASM → ranked candidates | ✅ verified in browser |

## Deviations from Plan (IMPORTANT — scope drift into Phase 2)

The plan called for DOM-based result display only (candidate count + top metrics as text).
The implementation went **far further**, pulling in most of Phase 2's scope:

- Three.js viewport with orbit/pan/zoom (planned in 02-01)
- Multi-worker candidate scoring architecture (not in any plan)
- Build plate visualization + overhang heatmap coloring (planned in 03-02)
- Binary STL export with quaternion application (planned in 02-03)
- Mesh decimation for scoring speed (planned in 03-01)

**Why:** Once the WASM boundary was set (see 01-02-SUMMARY), building the viewport and
worker pipeline was the natural next step to *see* the candidates. The DOM-only display
was skipped as an interim and replaced by the real viewport.

**Impact:** Phase 2's plans (02-01 viewport, 02-02 yaw, 02-03 export) are substantially
implemented already. Phase 2 tracking must reflect this — see STATE.md. Phase 3 items
(multi-worker, decimation, heatmap) also partially landed here.

## Open Issues (carried to Phase 2)

- Candidate poses are not consistently centered in the viewport above the floor —
  some candidates render off-center or below the build plate grid. See pending todo
  `bug-candidate-centering.md`. Root cause suspected: lift/centering logic in
  `viewport.ts` needs to account for the rotated bounding box, not the original.
- Quaternion composition (`qYaw * qAlign(dir, -Y)`) was added last session but
  needs final visual verification per candidate.

## Commands

```bash
cd web && npm run dev    # dev server at localhost:5173
```
