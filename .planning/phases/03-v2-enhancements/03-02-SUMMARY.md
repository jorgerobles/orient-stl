# Plan 03-02 SUMMARY: Interactive overlay

**Phase:** 03 (v2-enhancements)
**Plan:** 02
**Status:** Complete (pending human verification of Task 3)

## What was built

### Task 1: Height-weighted consensus ranking + hull+sphere toggle

- **Height-weighted overhang penalty** in `web/src/compute.ts:rankByConsensus()` — normalised overhang penalty is multiplied by `(1 + normalisedHeight * 0.5)` with fixed k=0.5 before computing the consensus minmax score. Tall overhangs penalised more than short ones.

- **`nearestCandidateScore()`** in `web/src/compute.ts` — quaternion-based angular distance scoring for overlay drag. Given a quaternion, finds the nearest pre-computed candidate direction and returns its composite score. Pure math (no THREE.js dependency in worker context).

- **`types.ts` mode extended** — `OrientConfig.mode` now accepts `"hull" | "hull_plus_sphere"`. Default stays `"hull"`.

- **Hull+Sphere checkbox** in `web/index.html` and `web/src/main.ts` — checkbox in config panel toggles `config.mode`. Stores `lastFile` reference; on toggle change, re-spawns compute with updated mode.

### Task 2: Overlay drag-to-rotate mode + live score badge

- **`Viewport.enterOverlayMode()`** in `web/src/viewport.ts` — disables OrbitControls, adds pointer event handlers for Arcball-style drag rotation. Computes camera-relative rotation axes. Fires `onOrientationChange` callback on every pointer move with current quaternion `[x, y, z, w]`.

- **`Viewport.exitOverlayMode()`** in `web/src/viewport.ts` — re-enables OrbitControls, removes pointer event listeners. Mesh quaternion stays at last drag position.

- **`Viewport.getMeshQuaternion()`** — returns current mesh quaternion.

- **Overlay orchestration** in `web/src/main.ts` — `enterOverlay()` creates backdrop div, toolbar with score badge, Varita Mágica button, and Exit button. Hides prev/next nav and yaw panel. `exitOverlay()` cleans up DOM and re-shows controls. Esc key and Exit button both exit.

- **Candidate list click** now enters overlay mode instead of just showing the candidate.

### Task 3: Varita Mágica button wiring

- **Varita Mágica button** in overlay toolbar — click handler extracts current direction from mesh quaternion, calls WASM `refine_orientation()` with full mesh data, computes new quaternion from refined direction, updates viewport and score badge.

- **Button state management** — disabled during compute with "Refining..." text; re-enabled on completion or error. Error shows "Error" for 3 seconds then reverts to previous score.

## Verification

| Check | Result |
|-------|--------|
| `npx tsc --noEmit` | ✅ 0 errors |
| `npx vitest run` | ✅ 25/25 tests pass |
| `npx vite build` | ✅ Build succeeds |

## Deviations from plan

- WASM `refine_orientation` import uses dynamic `import()` + `(wasmModule as any)` cast since the auto-generated TypeScript types don't include the new function yet (wasm-pack bindings). This is safe at runtime — the function exists in the compiled WASM binary.
- THREE.js `Quaternion.setFromUnitVectors` used for direction-to-quaternion conversion after refine, avoiding manual quaternion alignment code duplication.
