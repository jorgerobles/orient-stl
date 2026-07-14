---
phase: 06-frontend-architecture-refactor
plan: 02
subsystem: frontend-state-viewport
tags: appstate, eventtarget, viewport, gizmo, draphandler, camerarig, refactoring, tdd

# Dependency graph
requires:
  - phase: 06-01-foundation
    provides: types.ts, constants.ts, CSS extraction
provides:
  - AppState store with EventTarget-based subscribe/notify (criterion C2)
  - Viewport decomposed into GizmoController, DragHandler, CameraRig (criterion C3)
  - Unit test coverage for AppState (6 tests), DragHandler (4 tests), GizmoController (3 tests) (criterion C10-viewport)
  - Pitfall 3 axis-mapping behavior pinned by regression test
affects: [06-03, 06-04]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "AppState: hand-rolled EventTarget subclass for single mutable store"
    - "GizmoController: owns its own THREE.Group, billboard/raycast/hover encapsulated"
    - "DragHandler: pointer capture + angle-delta rotation math, consumes GizmoController.raycastRing"
    - "CameraRig: minimal camera positioning extracted per criteria"
    - "Barrel re-export: index.ts preserves main.ts import path"
    - "Pitfall 3 regression pin: axis-mapping test written BEFORE extraction"

key-files:
  created:
    - web/src/app/AppState.ts
    - web/src/app/AppState.test.ts
    - web/src/viewport/GizmoController.ts
    - web/src/viewport/GizmoController.test.ts
    - web/src/viewport/CameraRig.ts
    - web/src/viewport/DragHandler.ts
    - web/src/viewport/DragHandler.test.ts
    - web/src/viewport/Viewport.ts
    - web/src/viewport/index.ts
  modified: []
  deleted:
    - web/src/viewport.ts

key-decisions:
  - "AppState uses EventTarget (browser-native, zero deps) instead of nanostores/valtio — ponytail-compliant"
  - "AppState exported as class only, no singleton — always injected via constructor (testability)"
  - "DragHandler.getAxisVector preserves current axis mapping: axis-y → world Z, axis-z → world Y (Pitfall 3 pin)"
  - "CameraRig extracted as minimal class (positionForBoundingBox + reset) per criteria despite ~9 lines — no animation/easing (YAGNI)"
  - "GizmoController owns its own THREE.Group; Viewport adds/removes from scene"

patterns-established:
  - "Pattern 6 (GizmoController): encapsulates ring geometry lifecycle, billboard, raycast, hover"
  - "Pattern 7 (DragHandler): pointer event lifecycle managed via bound arrow functions for clean dispose"
  - "Pattern 8 (CameraRig): thin wrapper around camera math, no scene graph responsibility"

requirements-completed: [C2, C3, C10]

# Metrics
duration: 3min
completed: 2026-07-14
---

# Phase 6 Plan 2: AppState Store + Viewport Decomposition

**AppState store with EventTarget-based subscribe/notify (6 tests), Viewport split into GizmoController + DragHandler + CameraRig (7 new tests), axis-mapping behavior pinned by regression test (Pitfall 3)**

## Performance

- **Duration:** 3 min
- **Started:** 2026-07-14T08:35:02Z
- **Completed:** 2026-07-14T08:38:24Z
- **Tasks:** 2 (4 TDD commits)
- **Files modified:** 9 created, 1 deleted, 0 modified

## Accomplishments

- AppState store with typed get/set/subscribe/unsubscribe using browser-native EventTarget (11 fields, zero deps)
- GizmoController: ring creation (X/Y/Z rings with RING_SCALE, camera ring with CAMERA_RING_SCALE), billboard animation, raycastRing hit detection, setHover opacity highlighting, dispose with geometry cleanup
- CameraRig: positionForBoundingBox using CAMERA_DIST_MULT + reset via Box3.setFromObject (minimal extraction)
- DragHandler: pointer capture with bound arrow functions, getNDC/intersectRingPlane/angleAroundAxis math, onDown/onMove/onUp handlers, dispose removes all 3 listeners
- Viewport reduced from 498 lines to ~180 lines (scene setup, render loop, mesh lifecycle, delegates to 3 sub-controllers)
- Pitfall 3 regression test (DragHandler.test.ts): axis-y → [0,0,1] (world Z), axis-z → [0,1,0] (world Y) — written BEFORE extraction
- Barrel `index.ts` preserves `import { Viewport } from './viewport'` path in main.ts
- 13 new tests: 6 AppState + 4 DragHandler + 3 GizmoController (48 total, +37%)

## Task Commits

Each task was committed atomically following TDD (RED → GREEN):

### Task 1: AppState Store (TDD)

| Phase | Commit | Hash | Description |
|-------|--------|------|-------------|
| RED | `test(06-02): add failing test for AppState store` | `3ac812b` | 6 tests: subscribe/notify, unsubscribe, get/set, immutability, constructor |
| GREEN | `feat(06-02): implement AppState store` | `fdd611c` | AppState extends EventTarget with typed get/set/subscribe |

### Task 2: Viewport Decomposition (TDD)

| Phase | Commit | Hash | Description |
|-------|--------|------|-------------|
| RED | `test(06-02): add DragHandler axis-mapping regression test (Pitfall 3 pin)` | `660755c` | 4 tests: getAxisVector for axis-x/y/z + dispose cleanup |
| GREEN | `feat(06-02): decompose Viewport into GizmoController + CameraRig + DragHandler` | `bb65afa` | 7 files: GizmoController, CameraRig, DragHandler, slimmed Viewport, barrel, GizmoController tests |

## Files Created/Deleted

### Created (9 files)
- `web/src/app/AppState.ts` — AppState class (extends EventTarget)
- `web/src/app/AppState.test.ts` — 6 unit tests
- `web/src/viewport/GizmoController.ts` — GizmoController + RingAxis type
- `web/src/viewport/GizmoController.test.ts` — 3 unit tests (raycast, hover, dispose)
- `web/src/viewport/CameraRig.ts` — CameraRig class (positionForBoundingBox, reset)
- `web/src/viewport/DragHandler.ts` — DragHandler class (pointer capture, angle math, getAxisVector)
- `web/src/viewport/DragHandler.test.ts` — 4 unit tests (including Pitfall 3 pin)
- `web/src/viewport/Viewport.ts` — Slimmed Viewport (~180 lines from 498)
- `web/src/viewport/index.ts` — Barrel re-export

### Deleted (1 file)
- `web/src/viewport.ts` — Replaced by viewport/ directory

## Decisions Made

- **AppState via EventTarget**: Used browser-native EventTarget (~30 lines) instead of nanostores/valtio. Ponytail-compliant, zero dependencies, only one store needed.
- **No AppState singleton**: Store is always injected via constructor per research anti-pattern guidance. Testability.
- **Pitfall 3 axis mapping preserved**: DragHandler.getAxisVector returns the exact current behavior (axis-y → world Z, axis-z → world Y) with an explicit comment documenting it as intentional. The regression test (written first in RED phase) pins this behavior.
- **CameraRig extraction**: Despite being only ~9 lines of salvageable logic, extracted as mandated by criteria. No animation/easing added (YAGNI).
- **GizmoController owns THREE.Group**: Viewport adds/removes the group from scene but does not own ring lifecycle directly.
- **Bound arrow functions for dispose**: DragHandler stores arrow function references as class properties so dispose() can removeEventListener with the exact same references.

## Deviations from Plan

None — plan executed exactly as written.

## Test Results

| Suite | Tests | Status |
|-------|-------|--------|
| AppState.test.ts | 6 | ✅ Pass |
| DragHandler.test.ts | 4 | ✅ Pass |
| GizmoController.test.ts | 3 | ✅ Pass |
| Existing (35 tests, unchanged) | 35 | ✅ Pass |
| **Total** | **48** | **✅ 100% Pass** |

## Threat Model Compliance

- **T-06-03 (Tampering, subscribe/unsubscribe lifecycle)**: AppState.subscribe returns an unsubscribe function. Each view's dispose() must call it. Not yet enforced (Plan 03).
- **T-06-04 (Repudiation, silent axis-mapping change)**: DragHandler.test.ts pins axis-y → world Z mapping. Any change breaks the test explicitly.
- **T-06-05 (Denial of Service, listener cleanup)**: DragHandler.dispose() removes all 3 pointer event listeners. GizmoController.dispose() removes all children from its group. Viewport.dispose calls both.

## Self-Check: PASSED

- ✅ `ls web/src/viewport.ts` fails (file deleted)
- ✅ `ls web/src/viewport/Viewport.ts` succeeds (directory created)
- ✅ GizmoController class exists in GizmoController.ts
- ✅ CameraRig class exists in CameraRig.ts
- ✅ DragHandler class exists in DragHandler.ts
- ✅ getAxisVector present in DragHandler.ts
- ✅ '0, 0, 1' (axis-y → world Z) present in DragHandler.ts
- ✅ '0, 1, 0' (axis-z → world Y) present in DragHandler.ts
- ✅ "Pitfall 3" comment present in DragHandler.ts
- ✅ billboard method present in GizmoController.ts
- ✅ raycastRing method present in GizmoController.ts
- ✅ positionForBoundingBox method present in CameraRig.ts
- ✅ export * Viewport in index.ts
- ✅ GizmoController referenced in Viewport.ts
- ✅ CameraRig referenced in Viewport.ts
- ✅ DragHandler referenced in Viewport.ts
- ✅ attachPointerHandlers NOT present in Viewport.ts (count: 0)
- ✅ `npx vitest run` — 48 tests passing
- ✅ `npx tsc --noEmit` — 0 errors
- ✅ `npm run build` — succeeds
- ✅ Zero .rs files touched

---

*Phase: 06-frontend-architecture-refactor*
*Completed: 2026-07-14*
