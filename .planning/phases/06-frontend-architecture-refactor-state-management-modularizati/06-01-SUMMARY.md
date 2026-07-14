---
phase: 06-frontend-architecture-refactor
plan: 01
subsystem: frontend-foundation
tags: types, constants, css-extraction, refactoring, vite, web-worker

# Dependency graph
requires:
  - phase: 05-scoring-expansion-refinement
    provides: compute.ts (decimateForScore), type structure
provides:
  - Centralized types.ts with all shared data types and worker message contracts
  - constants.ts with named constants replacing magic numbers
  - Stripped compute.ts containing only decimateForScore
  - profiles/index.ts now exports WEIGHT_PRESETS
  - styles/theme.css with CSS custom properties
  - styles/main.css with global layout and component styles
  - index.html with zero inline CSS
affects: [06-02, 06-03, 06-04]

# Tech tracking
tech-stack:
  added:
    - web/src/styles/theme.css (CSS custom properties)
    - web/src/styles/main.css (global stylesheet)
  patterns:
    - Centralized type definitions in types.ts
    - Named constants in dedicated constants.ts
    - CSS custom properties for theming
    - Vite CSS imports for stylesheet bundling

key-files:
  created:
    - web/src/constants.ts
    - web/src/styles/theme.css
    - web/src/styles/main.css
  modified:
    - web/src/types.ts
    - web/src/compute.ts
    - web/src/centering.ts
    - web/src/centering.test.ts
    - web/src/convention.ts
    - web/src/convention.test.ts
    - web/src/orient.worker.ts
    - web/src/nearestScore.ts
    - web/src/profiles/index.ts
    - web/src/main.ts
    - web/src/loadSTL.ts
    - web/index.html

key-decisions:
  - "WorkerMessage/WorkerRequest defined but NOT applied to worker/handler until Plan 03 (avoids premature coupling)"
  - "CSS custom properties use exact extracted values from original inline styles for pixel-perfect equivalence"
  - "WEIGHT_PRESETS moved to profiles/index.ts alongside loadProfiles for single point of import"
  - "MAX_FILE_BYTES kept in constants.ts even though loadSTL.ts is the only consumer (centralized constant registry)"
  - "liftOntoPlate, SliceResult, RefineFn deleted with zero remaining references"

patterns-established:
  - "Pattern 1: Shared types in types.ts, not compute.ts"
  - "Pattern 2: All magic numbers and string literals in constants.ts"
  - "Pattern 3: WEIGHT_PRESETS lives in profiles/index.ts (co-located with profile loading)"
  - "Pattern 4: CSS split into theme.css (variables) and main.css (rules)"
  - "Pattern 5: index.html contains only structural markup, no inline CSS"

requirements-completed: [C4, C5, C9, C11]

# Metrics
duration: 12 min
completed: 2026-07-14
---

# Phase 6 Plan 1: Foundation Layer — Types, Constants, CSS Extraction

**Centralized all shared types into types.ts, created constants.ts with named constants, stripped compute.ts to decimateForScore only, deleted dead symbols (liftOntoPlate, SliceResult, RefineFn), and extracted 180 lines of inline CSS from index.html into a styles/ directory with CSS custom properties.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-07-14T10:28:00Z
- **Completed:** 2026-07-14T10:32:00Z
- **Tasks:** 2
- **Files modified:** 15 (3 created, 12 modified)

## Accomplishments

- Moved OriData, Candidate, ComputeConfig, ScoreWeights from compute.ts to types.ts
- Added WorkerMessage and WorkerRequest discriminated-union types (defined, not yet consumed)
- Created constants.ts with 20 named exports replacing magic numbers
- Moved WEIGHT_PRESETS to profiles/index.ts (co-located with loadProfiles)
- Deleted liftOntoPlate from centering.ts and SliceResult/RefineFn from compute.ts
- Removed 3 liftOntoPlate tests (now 35 total)
- Updated all imports across 7 files to reference new locations
- Created styles/theme.css with 14 CSS custom properties on :root
- Created styles/main.css with ~180 lines of global styles (with var() references)
- Stripped the entire 176-line <style> block from index.html
- Added Vite CSS imports in main.ts for both stylesheets

## Task Commits

Each task was committed atomically:

1. **Task 1: Remove liftOntoPlate tests** (TDD RED) - `9114cb0` (test)
2. **Task 1: Types/constants/imports refactor** (TDD GREEN) - `b7e536d` (feat)
3. **Task 2: CSS extraction** - `d41433c` (feat)

## Files Created/Modified

### Created
- `web/src/constants.ts` — 20 named constants (DECIMATE_TARGET, STORAGE_KEY, SCHEMA_VERSION, MIN_ANGLE_DEG, DEFAULT_REFINE_SEED, DEFAULT_PROFILE, DEFAULT_RANKER, METRIC_STRIDE, MAX_FILE_BYTES, camera/viewport constants)
- `web/src/styles/theme.css` — CSS custom properties (:root variables for colors, spacing, typography)
- `web/src/styles/main.css` — Global layout, typography, buttons, progress, config, results, score, scrollbar styles + .sr-only

### Modified
- `web/src/types.ts` — Gained OriData, Candidate, ComputeConfig, ScoreWeights, WorkerMessage, WorkerRequest (was only OrientConfig + defaultConfig)
- `web/src/compute.ts` — Stripped to only decimateForScore (removed 5 type definitions, WEIGHT_PRESETS)
- `web/src/centering.ts` — Removed liftOntoPlate function
- `web/src/centering.test.ts` — Removed liftOntoPlate import and describe block (35 tests from 38)
- `web/src/convention.ts` — Updated comment reference liftOntoPlate → centroidTranslate
- `web/src/convention.test.ts` — Updated comment reference liftOntoPlate → centroidTranslate
- `web/src/orient.worker.ts` — Import from './types' instead of './compute'
- `web/src/nearestScore.ts` — Import from './types' instead of './compute'
- `web/src/profiles/index.ts` — Now exports WEIGHT_PRESETS directly, imports from '../types'
- `web/src/main.ts` — Updated imports, replaced magic numbers with named constants, added CSS imports
- `web/src/loadSTL.ts` — Import from './types' instead of './compute', uses MAX_FILE_BYTES
- `web/index.html` — Removed entire 176-line <style> block

## Threat Model Compliance

- **T-06-01 (Tampering, localStorage schema):** SCHEMA_VERSION check preserved in loadConfig — STORAGE_KEY and SCHEMA_VERSION moved to constants.ts as named exports, logic unchanged. ✓

## Decisions Made

- WorkerMessage/WorkerRequest are defined but NOT wired to the worker or handler — that happens atomically in Plan 03 to avoid premature coupling
- CSS custom properties use exact hex/rgba values from the original inline styles to guarantee visual equivalence (no color drift)
- MAX_FILE_BYTES centralized in constants.ts despite being consumed by only loadSTL.ts, keeping the constants registry complete

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

None.

## Next Phase Readiness

- Foundation types established for AppState (Plan 02), Viewport decomposition (Plan 02), and AppController (Plan 03)
- constants.ts provides the vocabulary that all subsequent plans import from
- CSS extraction enables per-component CSS modules in Plan 03
- Ready for Plan 02 (state management + Viewport modularization)

## Self-Check: PASSED

- ✅ 3 new files exist on disk (constants.ts, theme.css, main.css)
- ✅ All 3 commits present in git log (9114cb0, b7e536d, d41433c)
- ✅ `npx tsc --noEmit` passes with 0 errors
- ✅ `npx vitest run` passes with 35 tests
- ✅ `npm run build` succeeds
- ✅ Zero dead code references (liftOntoPlate, SliceResult, RefineFn)
- ✅ Zero inline `<style>` in index.html
- ✅ Zero Rust files touched

---

*Phase: 06-frontend-architecture-refactor*
*Completed: 2026-07-14*
