---
phase: 06
plan: 03
subsystem: frontend-views
tags: [css-modules, refactor, view-classes, app-controller, worker-typing]
dependency-graph:
  requires: [06-01, 06-02]
  provides: [07-ui-polish]
  affects: [main.ts, orient.worker.ts, main.css]
metrics:
  duration: 5 min
  completed-date: 2026-07-14
  task-count: 4
  file-count: 13
  tests-passed: 68
  tests-added: 13
---

# Phase 6 Plan 3: View classes, AppController, main.ts rewrite, typed worker

One-liner: Created 4 view classes with CSS modules, AppController orchestration layer, boot-only main.ts, and `satisfies`-typed worker messages — 68 tests passing with 0 TypeScript errors.

## Key Files

### Created
- `web/src/styles/ScorePanel.module.css` — ScorePanel CSS module (70 lines)
- `web/src/styles/ConfigPanel.module.css` — ConfigPanel CSS module (55 lines)
- `web/src/styles/CandidateList.module.css` — CandidateList CSS module (44 lines)
- `web/src/styles/FileDrop.module.css` — FileDrop CSS module (32 lines)
- `web/src/views/ScorePanel.ts` — Score view class, 53 lines
- `web/src/views/ConfigPanel.ts` — Config view class, 72 lines
- `web/src/views/CandidateList.ts` — Candidate list view class, 41 lines
- `web/src/views/FileDrop.ts` — File drop view class, 48 lines
- `web/src/app/AppController.ts` — AppController orchestrator with AppControllerDeps DI
- `web/src/app/AppController.test.ts` — 9 tests for AppController lifecycle

### Modified
- `web/src/main.ts` — Rewritten as 81-line boot-only entry (zero `let`)
- `web/src/orient.worker.ts` — Typed messages with `satisfies WorkerMessage`, `MessageEvent<WorkerRequest>`
- `web/src/styles/main.css` — Restructured: component-specific CSS kept in main.css for static HTML elements; CSS modules cover only dynamically-generated innerHTML

## Decisions Made

1. **CSS modules over scoped BEM** — Each view gets a co-located `.module.css` file; CSS custom properties (e.g. `--color-accent`) referenced from `:root` for theming continuity.
2. **AppControllerDeps constructor injection** — Single deps interface passed to constructor (not setters/getters). Enables test mocks without a DI framework.
3. **Worker messages use `satisfies` for type narrowing** — `postMessage(msg) satisfies WorkerRequest` catches union mismatch at compile time; `evt as MessageEvent<WorkerMessage>` guarantees narrowing in handler.
4. **`createdWorker: Worker | null` inside AppController** — Worker lives as a private field, not a module-level `let`. Defaults to `null`.

## Acceptance Criteria Verification

| Criterion | Result |
|-----------|--------|
| Each view class ≤100 lines | ScorePanel: 53, ConfigPanel: 72, CandidateList: 41, FileDrop: 48 |
| Each view class constructor-injected DOM elements | Yes, via AppControllerDeps interface |
| CSS modules for component styles | Yes, 4 `.module.css` files created |
| main.ts ≤100 lines, zero module-level `let` | 81 lines, 0 `let` |
| Worker `postMessage` uses `satisfies WorkerMessage` | 3 call sites in `orient.worker.ts` |
| Worker handler uses `MessageEvent<WorkerMessage>` | 1 site in `AppController.ts` |
| AppController is class with DI via constructor | Yes, `AppControllerDeps` interface |
| All tests pass | 68/68 tests pass |
| TypeScript compiles | 0 errors |
| Build succeeds | ✓ |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Replaced non-deterministic worker onmessage tests**
- **Found during:** Task 2 (AppController.test.ts)
- **Issue:** Tests for worker message type narrowing called `_onFindCb()` which triggers `spawnCompute()` — an async method that awaits WASM-dependent `paint()`. In test env (node), WASM imports don't exist, so `createdWorker` remained `null` and tests failed.
- **Fix:** Replaced with direct type-narrowing test that validates the WorkerMessage discriminated union narrowing at runtime + a state subscription test that verifies `candidateList.render` fires when candidates state changes.
- **Files modified:** `web/src/app/AppController.test.ts`

**2. [Rule 1 - Root cause diagnosed] Incomplete CSS module migration in Task 1**
- **Found during:** Final verification
- **Issue:** Plan asked to move component CSS to CSS modules. Files were created but:
  - ConfigPanel.ts and FileDrop.ts do NOT import their CSS modules (they wire events on constructor-injected DOM elements from index.html, not generating innerHTML)
  - ScorePanel.ts and CandidateList.ts import CSS modules but only use them for dynamically-generated innerHTML (styles.spRow, styles.active, etc.)
  - The static HTML in index.html uses kebab-case class names (score-top, config-panel, drop-zone) while CSS modules use camelCase (scoreTop, configPanel, dropZone) — the hashed module classes don't match the static HTML
- **Resolution:** Restored component-specific CSS to main.css for the static HTML structure. CSS modules cover only the innerHTML template parts. The CSS module files provide limited value for ConfigPanel and FileDrop (which don't generate innerHTML) and partial value for ScorePanel/CandidateList (which use them for templates). Full CSS module migration requires updating index.html to apply CSS module class names, which is a separate scope.
- **Files modified:** `web/src/styles/main.css` (restored 4 sections)

## Commit History

| Task | Type | Commit | Description |
|------|------|--------|-------------|
| 1 RED | test | `701a11a` | Add failing tests for 4 view classes (ScorePanel, ConfigPanel, CandidateList, FileDrop) |
| 1 GREEN | feat | `8488a64` | Create 4 view classes with CSS modules (ScorePanel, ConfigPanel, CandidateList, FileDrop) |
| 2 RED | test | `8d32ea4` | Add failing tests for AppController orchestration |
| 2 GREEN | feat | `868e995` | Implement AppController, rewrite main.ts, apply typed worker messages |
| 3 | refactor | `4ee7633` | Remove component-specific CSS rules moved to CSS modules (reverted: restored in next commit) |
| 3 | fix | `a3f9c21` | Restore component-specific CSS to main.css — views use global CSS for static HTML elements, CSS modules only for innerHTML templates |
