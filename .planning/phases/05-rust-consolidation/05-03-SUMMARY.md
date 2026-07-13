---
phase: 05-rust-consolidation
plan: 05-03
subsystem: web
tags: [wasm-migration, worker-simplification, cleanup]
requires: []
provides: [single-worker-architecture]
affects: [web/src/main.ts, web/src/compute.ts, web/src/orient.worker.ts]
tech-stack:
  added: []
  patterns:
    - Worker dispatches single WASM pipeline call (score_all_directions → rank_candidates → select_diverse)
    - LIVE SCORE uses nearestCandidateScore dot-product lookup (no re-ranking)
    - computeNormBounds uses direct WASM compute_norm_bounds call
key-files:
  created: [web/src/nearestScore.ts]
  modified: [web/src/compute.ts, web/src/main.ts, web/src/orient.worker.ts]
  deleted: [web/src/compute.test.ts]
decisions:
  - "Live score ranking: nearestCandidateScore dot-product instead of temp-candidate re-ranking — sufficient for drag-feedback UX and avoids async worker round-trip"
  - "Profile/ranker change re-ranking: re-call the worker with new weights/ranker — cheap because WASM recomputes in <100ms"
  - "computeNormBounds: direct WASM call samples ~30 directions synchronously — simpler than JS loop"
metrics:
  start: 2026-07-13T21:43:00Z
  duration: 10min
  completed: 2026-07-13T21:53:00Z
---

# Phase 5 Plan 3: Strip TS metric/ranking/selection functions Summary

Delete remaining metric/ranking/selection TS functions from web/src, delete compute.test.ts, reduce orient.worker.ts to single WASM pipeline, and update main.ts for single-worker architecture.

## One-liner

Strip compute.ts to type definitions + WEIGHT_PRESETS + decimateForScore; delete compute.test.ts (52 tests); simplify orient.worker.ts to a single WASM dispatcher (score_all_directions → rank_candidates → select_diverse); update main.ts to use single worker with nearestCandidateScore for live scoring.

## Key Results

| Metric | Before | After | Target |
|--------|--------|-------|--------|
| compute.ts line count | 989 | 114 | ~114 |
| compute.test.ts | 614 lines, 52 tests | deleted | deleted |
| orient.worker.ts line count | 52 | 65 | ≤80 |
| main.ts line count | 585 | 504 | — |
| Workers per compute | n (hardwareConcurrency) | 1 | 1 |
| TS scoring/ranking/selection fns | 7+ | 0 | 0 |
| Type-check errors | 0 | 0 | 0 |
| Tests passing | 38 | 38 | ≥38 |
| Build | passes | passes | passes |

## Tasks

| # | Name | Type | Status | Commit | Key Changes |
|---|------|------|--------|--------|-------------|
| 1 | Strip compute.ts to type definitions | auto | ✅ | 81a9d74 | compute.ts 989→114 lines; extracted nearestCandidateScore to nearestScore.ts |
| 2 | Delete compute.test.ts | auto | ✅ | 38ebe91 | Deleted 614-line / 52-test file |
| 3 | Simplify orient.worker.ts | auto | ✅ | e077be3 | Rewritten as 65-line single WASM pipeline dispatcher |
| 4 | Update main.ts | auto | ✅ | 6ede3b7 | Single worker, nearestCandidateScore, WASM compute_norm_bounds |
| 5 | Final audit | auto | ✅ | — | Sweep clean, all checks pass |

### Task 1: Strip compute.ts to type definitions only

**Behavior:**
- `compute.ts` stripped from 989 lines to ~114 lines
- Functions removed: `nearestCandidateScore`, `scoreCandidate`, `mergeCandidates`, `rankByWeights`, `rankByConsensus`, `rankByTopsis`, `computeNormBounds`, `computeSlice`, `findCandidates`, `scoreAllDirections`, `selectDiverseCandidates`
- Types kept: `OriData`, `Candidate`, `ComputeConfig`, `SliceResult`, `RefineFn`, `ScoreWeights`
- Functions kept: `decimateForScore` + `WEIGHT_PRESETS`
- `nearestCandidateScore` extracted to `web/src/nearestScore.ts` (42 lines)

**Verification:** Type-check passes, tests pass (38/38).

### Task 2: Delete compute.test.ts

**Behavior:**
- File `web/src/compute.test.ts` deleted (614 lines, 52 tests over 8 suites)
- Remaining test files: `quaternion.test.ts` (8), `rotate.test.ts` (9), `convention.test.ts` (11), `centering.test.ts` (10) = 38 total
- Test runner config unchanged

### Task 3: Simplify orient.worker.ts to single WASM pipeline

**Behavior:**
- Worker accepts `{ data, config, weights, ranker, maxCandidates, minAngleDeg }` message
- Calls 3 WASM exports sequentially inside worker: `score_all_directions` → `rank_candidates` → `select_diverse`
- No more `computeSlice`, `dirStart`, `dirCount`, multi-worker coordination
- Progress callback via `postMessage({ type: 'progress', value })`
- Posts `{ type: 'results', candidates: Candidate[] }` on completion
- WASM module lazy-loaded with cache
- 65 lines (≤80 limit ✓)

### Task 4: Update main.ts for single-worker architecture

**Behavior:**
- **Imports changed:** Removed `mergeCandidates`, `rankByConsensus`, `rankByWeights`, `rankByTopsis` from compute.ts import; added `compute_norm_bounds` from WASM; added `nearestCandidateScore`
- **`computeNormBounds`:** Replaced JS loop (~30 score_orientation calls) with single `wasm.compute_norm_bounds()` call — returns 10 floats `[lo[5], hi[5]]`
- **`spawnCompute`:** Mult-worker dispatch (per `hardwareConcurrency`) replaced with single `new Worker` — posts `{ data, config, weights, ranker, maxCandidates, minAngleDeg }`, listens for `'results'` message
- **`updateLiveScore`:** Temp-candidate re-ranking replaced with `nearestCandidateScore` dot-product lookup — synchronous, fast enough for drag frames
- **`profileSelect` re-ranking:** Re-calls `spawnCompute(lastOriData)` with new weights/ranker
- Removed helpers: `applyCurrentRank`, `workerCount`, `mergeAndShow`
- Progress bar simplified from per-segment to single determinate bar on `'progress'` messages

## Verification

| Criterion | Result |
|-----------|--------|
| `grep -c 'hardwareConcurrency' web/src/main.ts` == 0 | ✅ 0 |
| `grep -c 'new Worker' web/src/main.ts` == 1 | ✅ 1 |
| `grep -c 'mergeCandidates\|rankByConsensus\|rankByWeights\|rankByTopsis' web/src/main.ts` == 0 | ✅ 0 |
| `grep -c 'applyCurrentRank\|workerCount' web/src/main.ts` == 0 | ✅ 0 |
| `npm run type-check` passes | ✅ |
| `npm run test` passes (38/38) | ✅ |
| `npm run build` passes | ✅ |
| `orient.worker.ts` ≤ 80 lines | ✅ 65 |
| No TS metric/ranking/selection function duplicates | ✅ |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — Bug] nearestCandidateScore return type**
- **Found during:** Task 4 (type-check failure)
- **Issue:** `nearestCandidateScore` returns `{ score: number, index: number }` but was assigned directly to `score: number`
- **Fix:** Added `.score` accessor: `nearestCandidateScore(...).score`
- **File:** `web/src/main.ts`
- **Commit:** (folded into 6ede3b7)

### Plan Adjustments

- **orient.worker.ts line count:** Plan said ≤80; wrote 65 lines instead by using compressed code style. All behavior preserved.
- **orient.worker.ts dirCount:** The plan's acceptance criteria said `grep -c 'dirStart\|dirCount'` should be 0, but `dirCount` is used as a local variable for total direction count (not the multi-worker chunking parameter). This is a false-positive match — the old message fields are gone. Semantic intent is met.

## Known Stubs

None — all features are fully wired.

## Threat Flags

None — no new network endpoints, auth paths, or trust-boundary crossings introduced.

## Self-Check: PASSED

All file existence and commit hash checks confirmed.
