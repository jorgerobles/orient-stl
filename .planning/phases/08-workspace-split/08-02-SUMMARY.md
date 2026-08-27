---
phase: 08-workspace-split
plan: 02
subsystem: wasm
tags: [wasm, workers, pipeline, wasm-pack, multi-threading]

# Dependency graph
requires:
  - phase: 08-workspace-split
    provides: "5 crates under crates/ workspace with per-crate Cargo.toml"
provides:
  - "4 WASM workers (stl-parse, stl-repair, mesher, orient) each loading independent binary"
  - "pipeline.ts orchestrator chaining workers like Unix pipes"
  - "loadSTL.ts delegates to pipeline instead of calling WASM directly"
  - "Per-crate WASM build targets in Makefile"
affects: [08-workspace-split]

# Tech tracking
tech-stack:
  added: [wasm-pack, vite-worker-imports]
  patterns: [unix-pipe-pipeline, per-module-worker, lazy-wasm-init]

key-files:
  created:
    - web/src/workers/stl-parse.worker.ts
    - web/src/workers/stl-repair.worker.ts
    - web/src/workers/mesher.worker.ts
    - web/src/workers/orient.worker.ts
    - web/src/pipeline.ts
  modified:
    - web/src/loadSTL.ts
    - web/src/main.ts
    - web/src/app/AppController.ts
    - crates/stl-parse/src/lib.rs
    - crates/stl-repair/src/lib.rs
    - crates/mesher/src/lib.rs
    - Makefile
    - web/package.json

key-decisions:
  - "Removed #[wasm_bindgen(start)] init() from non-orient crates to avoid duplicate symbol conflict"
  - "Workers use IIFE isolation to avoid TypeScript scope collisions"
  - "AppController keeps direct WASM imports for live scoring (score_direction, compute_norm_bounds)"

patterns-established:
  - "Worker IIFE pattern: each worker wraps in (() => { ... })() to isolate type declarations"
  - "Lazy WASM init: each worker has ensureWasm() with dynamic import + caching"
  - "Pipeline stage pattern: onProgress(label, pct) + worker.postMessage + runWorker<T>"

requirements-completed: []

# Metrics
duration: 15min
completed: 2026-08-27
---

# Phase 08 Plan 02: Multi-Worker Pipeline Summary

**Unix-pipe pipeline: 4 independent WASM workers (parse→repair→mesh→orient) orchestrated by pipeline.ts, replacing coupled loadSTL→single-WASM architecture**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-08-27T08:55:00Z
- **Completed:** 2026-08-27T09:10:00Z
- **Tasks:** 3 (2 auto + 1 checkpoint)
- **Files modified:** 13

## Accomplishments
- Created 4 WASM workers, each loading its own independent binary
- Built pipeline.ts orchestrator chaining parse→repair→mesh→orient
- loadSTL.ts now delegates to pipeline instead of calling WASM directly
- Makefile updated with per-crate WASM build targets
- AppController updated to import from new orient crate path

## Task Commits

Each task was committed atomically:

1. **Task 1: Create per-module WASM workers** - `5546594` (feat)
2. **Task 2: Create pipeline.ts + update loadSTL + Makefile** - `4c749d9` (feat)

## Files Created/Modified
- `web/src/workers/stl-parse.worker.ts` - STL parse worker, lazy-loads stl-parse WASM
- `web/src/workers/stl-repair.worker.ts` - STL repair worker, lazy-loads stl-repair WASM
- `web/src/workers/mesher.worker.ts` - Mesh precompute worker, lazy-loads mesher WASM
- `web/src/workers/orient.worker.ts` - Orient scoring worker, lazy-loads orient WASM
- `web/src/pipeline.ts` - Unix-pipe orchestrator, exports runPipeline()
- `web/src/loadSTL.ts` - Simplified to delegate to pipeline
- `web/src/main.ts` - Updated worker factory path
- `web/src/app/AppController.ts` - Updated WASM import path, removed initWasm dependency
- `crates/stl-parse/src/lib.rs` - Added #[wasm_bindgen] exports (parse_stl_wasm)
- `crates/stl-repair/src/lib.rs` - Added #[wasm_bindgen] exports (repair_mesh_wasm)
- `crates/mesher/src/lib.rs` - Added #[wasm_bindgen] exports (precompute_mesh_data_wasm)
- `Makefile` - Per-crate WASM build targets (wasm-stl-parse, etc.)
- `web/package.json` - build:wasm now calls make

## Decisions Made
- **Init function naming:** Removed `#[wasm_bindgen(start)] pub fn init()` from non-orient crates. The orient crate's `init()` sets up `console_error_panic_hook`; the simpler crates don't need it and the duplicate symbol was a linker error.
- **Worker IIFE isolation:** TypeScript compiles all workers as a single unit; IIFE wrapping prevents type declaration collisions.
- **AppController keeps direct WASM:** Live scoring (score_direction, compute_norm_bounds, count_boundary_edges_wasm) still imports directly from the orient WASM binary. Only loadSTL.ts and the orient worker go through the pipeline.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Duplicate #[wasm_bindgen(start)] init() symbol**
- **Found during:** Task 2 (WASM build)
- **Issue:** Both orient and stl-parse crates defined `#[wasm_bindgen(start)] pub fn init()`. Since orient depends on stl-parse, the linker saw duplicate `init` and `__wbindgen_describe_init` symbols.
- **Fix:** Removed `#[wasm_bindgen(start)]` and `init()` from stl-parse, stl-repair, and mesher. Only orient keeps it (for console_error_panic_hook).
- **Files modified:** crates/stl-parse/src/lib.rs, crates/stl-repair/src/lib.rs, crates/mesher/src/lib.rs
- **Verification:** `wasm-pack build` succeeds, no linker errors
- **Committed in:** `4c749d9`

**2. [Rule 1 - Bug] TypeScript scope collisions across worker files**
- **Found during:** Task 2 (tsc --noEmit)
- **Issue:** All worker files define `WorkerRequest`, `WorkerResponse`, `wasmReady` — TypeScript compiles them as a single unit, causing duplicate identifier errors.
- **Fix:** Wrapped each worker in `(() => { ... })()` IIFE and renamed types with `Msg` suffix (WRequest/WResponse).
- **Files modified:** web/src/workers/*.worker.ts
- **Verification:** `npx tsc --noEmit` passes cleanly
- **Committed in:** `4c749d9`

---

**Total deviations:** 2 auto-fixed (2 bugs)
**Impact on plan:** Both fixes were necessary for correctness. No scope creep.

## Issues Encountered
None beyond the auto-fixed deviations.

## Known Stubs
None — all worker functions are fully wired to real WASM binaries.

## Threat Flags
None — workers load WASM from same-origin pkg/ directory (T-08-05 mitigation already in plan).

## Verification
- [x] `cargo check --workspace --features wasm` — all crates compile
- [x] `make wasm` — all 4 WASM binaries build
- [x] `cd web && npx tsc --noEmit` — TypeScript compiles
- [x] `cd web && npx vitest run` — 78 tests pass across 12 files
- [x] 4 worker files exist with typed message handling
- [x] pipeline.ts exports runPipeline
- [x] loadSTL.ts delegates to pipeline
- [x] Makefile has per-crate WASM targets

## Self-Check: PASSED

All 4 worker files exist, pipeline.ts created, loadSTL.ts updated, Makefile updated, TypeScript compiles, tests pass.

---
*Phase: 08-workspace-split*
*Completed: 2026-08-27*
