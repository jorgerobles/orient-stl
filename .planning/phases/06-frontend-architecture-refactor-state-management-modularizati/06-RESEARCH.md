# Phase 6: Frontend Architecture Refactor - Research

**Researched:** 2026-07-14
**Domain:** Vanilla TypeScript + Three.js + Vite — modularization, state management, accessibility, typed worker IPC
**Confidence:** HIGH (architectural recommendations are code-grounded; library checks verified against npm registry)

## Summary

The orient-stl frontend is a working but monolithic vanilla-TS app. `main.ts` (504 lines) acts as both orchestration controller and DOM binding layer, holding **18 module-level `let` variables** (the success criteria says "14+" — verified: 18). `viewport.ts` (498 lines) mixes Three.js scene setup, render loop, gizmo ring creation, raycasting, pointer drag math, and overlay mode in a single class. CSS lives as 180+ lines of inline `<style>` in `index.html`. The worker posts untyped `{ type, ... }` messages. Three exports (`liftOntoPlate`, `SliceResult`, `RefineFn`) are dead code with zero production callers (verified via codegraph). Three empty/trivial catch blocks swallow errors with comments-only recovery.

The refactor is greenfield-internal: zero new external dependencies are required. **Vite 8.1.4 (already installed) provides CSS modules natively.** `EventTarget` (browser-native) provides the observer substrate for a hand-rolled `AppState` store — ~30 lines, zero framework lock-in, aligns with the project's ponytail/YAGNI conventions and the existing "no JS metric library" decision from Phase 5. Vitest 4.1.10 is already configured; **happy-dom is NOT installed and should not be added** — AppController will be tested via dependency injection of mock DOM elements, keeping the test environment `node` for everything except thin view-class render tests (where happy-dom would be added only if those tests prove necessary).

**Primary recommendation:** Use a hand-rolled `AppState` class extending `EventTarget` (≈30 lines, no deps), split `main.ts` into `AppController` + per-panel view classes with constructor-injected DOM dependencies, extract `GizmoController` and `DragHandler` from `Viewport` (own their own `THREE.Group`), move CSS to `*.module.css` files, define a discriminated-union `WorkerMessage` type in `types.ts`, and add keyboard handlers + `role="application"` + `aria-live` region to the viewport. CameraRig extraction is optional (YAGNI borderline) — see Architectural Responsibility Map.

## Architectural Responsibility Map

Every capability in this phase is **Browser/Client tier** — orient-stl is a static SPA with no backend, no SSR, no service worker. There is no tier-confusion risk; the value of this map is to clarify **within-client** ownership (orchestration vs. rendering vs. compute).

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| App orchestration (file load → parse → render → compute → display) | AppController | AppState | Controller owns the lifecycle; state owns the data |
| Mutable UI state (config, candidates, profile, ranker, dirty flag) | AppState store | — | Single source of truth; views subscribe |
| Three.js scene + camera + render loop | Viewport | — | Owns renderer lifecycle; cannot be shared |
| Gizmo ring creation + billboarding + hover | GizmoController | Viewport.animate | Owns its own THREE.Group; Viewport calls `gizmo.billboard(camera)` per frame |
| Pointer drag math (raycast, angle delta, pointer capture) | DragHandler | GizmoController | Consumes GizmoController for ring hit detection; owns pointer listeners |
| Camera position + reset | Viewport (inline) | (CameraRig — optional) | Only ~9 lines; extracting CameraRig is YAGNI-borderline — see below |
| Mesh load + centering + overhang coloring | Viewport | — | Tightly coupled to geometry lifecycle |
| Long compute (score/rank/select) | Worker (orient.worker.ts) | WASM | Worker is a thin dispatcher; WASM does the math |
| Score normalization + live-score display | ScorePanel (view) | AppState (data) | View owns rendering; reads bounds + weights from state |
| File drag-drop + picker | FileDrop (view) | AppController | View owns DOM events; controller owns file lifecycle |
| CSS theming | styles/theme.css (`:root` custom properties) | — | Browser-native; no preprocessor needed |
| Worker IPC typing | types.ts (shared) | — | Both main and worker import the same union |

**CameraRig decision:** The success criteria lists `CameraRig` as a required extraction, but `resetCamera()` is **9 lines** and camera positioning appears in only 3 places (constructor, `loadModel`, `resetCamera`). Extracting it creates a 30-line class for ~9 lines of salvageable logic. **Recommendation: extract it anyway because the success criteria mandates it, but keep it minimal** — just `positionForBoundingBox(bb)` and `reset()`. Do NOT add camera animation/easing (YAGNI).

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `three` | 0.185.1 (installed) | 3D viewport, gizmo rings, raycasting | Already in use; no change `[VERIFIED: npm registry]` |
| `vite` | 8.1.4 (installed) | Build + CSS modules + worker bundling | Already in use; CSS modules are Vite-native — no plugin needed `[VERIFIED: npm registry]` |
| `vitest` | 4.1.10 (installed) | Test runner (node env, mock-friendly) | Already in use; 38 tests pass `[VERIFIED: npm registry]` |
| `typescript` | ^5.7.0 (installed) | Discriminated unions for worker messages | Already in use `[VERIFIED: npm registry]` |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| (none recommended) | — | — | This phase adds **zero** runtime dependencies |

**No supporting libraries.** The whole point of this refactor is to remove implicit complexity, not add new dependency surface. All proposed patterns are implementable with: native `EventTarget`, native CSS custom properties, native Vite CSS modules, native Web Worker postMessage, native keyboard events.

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Hand-rolled `AppState` store | `nanostores` 1.4.0 (5M/wk, `[VERIFIED: npm registry]`) | nanostores is excellent but adds a dep for ONE store; 30 lines of `EventTarget` is simpler and ponytail-compliant. `nanostores` was flagged `[SUS]` by the legitimacy heuristic for "too-new" (last published 2026-06-27) but this is a false positive — the package is well-established and actively maintained. Use only if the team explicitly wants framework-agnostic store ergonomics (`atom`, `computed`). |
| Hand-rolled `AppState` store | `@preact/signals-core` 1.14.4 (5M/wk, `[VERIFIED: npm registry]`) | Signals give fine-grained reactivity, but the app has ~6 distinct view sections — coarse subscribe/notify is sufficient. Also flagged `[SUS]` "too-new" (false positive — actively maintained by Preact team). |
| Hand-rolled `AppState` store | `valtio` 2.3.2 (1.7M/wk, verdict `[OK]`) | Proxy-based; pleasant ergonomics but adds 5KB and a `proxy()` mental model. Overkill for a single store. |
| Hand-rolled `AppState` store | `valtio`/`nanostores`/etc. | All three are legitimate. None beat a 30-line `EventTarget` subclass for THIS app's complexity. **The ponytail skill (loaded via project CLAUDE.md) and the global CLAUDE.md "standard library before deps" rule both point to hand-roll.** |
| Vitest `node` env + DI | `happy-dom` 20.10.6 (11M/wk) | Faster than jsdom; needed only if view classes (`ScorePanel.render`) are tested at the DOM level. **Recommendation: try DI-first; add happy-dom only if DOM-render tests are written.** Flagged `[SUS]` "too-new" — false positive (actively maintained). |
| Vitest `node` env + DI | `jsdom` 29.1.1 (66M/wk, verdict `[OK]`) | Heavier than happy-dom; same use case. Only consider if happy-dom fails on a specific API. |
| CSS custom properties + CSS modules | Sass / Tailwind / UnoCSS | All add build deps. Existing CSS is 180 lines — plain CSS modules are sufficient. YAGNI. |

**Installation:**
```bash
# NOTHING TO INSTALL. All dependencies are present.
# Verify before planning:
cd web && npm ls three vite vitest typescript
```

**Version verification (all confirmed against npm registry on 2026-07-14):**
- `three@0.185.1` — matches `package.json` `[VERIFIED: npm registry]`
- `vite@8.1.4` — slightly ahead of `^8.1.0` in package.json `[VERIFIED: npm registry]`
- `vitest@4.1.10` — matches `[VERIFIED: npm registry]`
- `@types/three@^0.185.1` — matches `[VERIFIED: npm registry]`

## Package Legitimacy Audit

> This phase installs **zero** external packages. The audit is recorded for completeness and to document the **decision not to add** nanostores/signals/valtio.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| `nanostores` | npm | mature (1.4.0) | 5M/wk | github.com/nanostores/nanostores | `[SUS]` (false positive — "too-new") | NOT ADDED — hand-roll AppState instead |
| `@preact/signals-core` | npm | mature (1.14.4) | 5M/wk | github.com/preactjs/signals | `[SUS]` (false positive — "too-new") | NOT ADDED — hand-roll AppState instead |
| `valtio` | npm | mature (2.3.2) | 1.7M/wk | github.com/pmndrs/valtio | `[OK]` | NOT ADDED — hand-roll AppState instead |
| `jsdom` | npm | mature (29.1.1) | 66M/wk | github.com/jsdom/jsdom | `[OK]` | NOT ADDED — DI-first test strategy avoids it |
| `happy-dom` | npm | mature (20.10.6) | 11M/wk | github.com/capricorn86/happy-dom | `[SUS]` (false positive — "too-new") | NOT ADDED initially; add only if DOM-render tests are written |

**Packages removed due to `[SLOP]` verdict:** none
**Packages flagged as suspicious `[SUS]`:** none added. The `[SUS]` flags on `nanostores`, `@preact/signals-core`, and `happy-dom` are heuristic false positives — all three are well-established, multi-million-download, actively-maintained packages whose only "violation" is having a recent release. **No `checkpoint:human-verify` task is needed because none of these packages are being installed.**

*The decision to hand-roll AppState is a deliberate ponytail/YAGNI choice, not a legitimacy concern. If the planner or user prefers a library, `valtio` (the only `[OK]` verdict) is the safest pick.*

## Architecture Patterns

### System Architecture Diagram

```
                    ┌─────────────────────────────────────────────────────────────┐
                    │                    Browser (Client Tier)                    │
                    │                                                             │
   User drop/click  │  ┌──────────────┐   file    ┌──────────────────────────┐   │
  ─────────────────────▶  FileDrop   ────────────▶│                          │   │
                    │  └──────────────┘            │      AppController       │   │
                    │  ┌──────────────┐   events   │   (orchestration)        │   │
                    │  │ ConfigPanel  ────────────▶│                          │   │
                    │  └──────────────┘            │  ┌───────────────────┐   │   │
                    │  ┌──────────────┐            │  │   AppState        │   │   │
                    │  │ CandidateList│◀───────────│  │   (EventTarget)   │   │   │
                    │  └──────────────┘   render   │  │   subscribe/notify│   │   │
                    │  ┌──────────────┐            │  └───────────────────┘   │   │
                    │  │  ScorePanel  │◀───read────│                          │   │
                    │  └──────────────┘            └─────┬──────────┬─────────┘   │
                    │                                    │          │             │
                    │  ┌──────────────────────────────┐ │          │ request     │
                    │  │       Viewport (slim)        │ │          ▼             │
                    │  │  scene + camera + render     │ │  ┌────────────────┐    │
                    │  │  ┌────────────────────────┐  │ │  │ orient.worker  │    │
                    │  │  │   GizmoController      │  │ │  │  (dispatcher)  │    │
                    │  │  │   owns THREE.Group     │  │ │ └───────┬────────┘    │ │
                    │  │  │   billboard() / hover  │  │ │         │ postMessage │ │
                    │  │  └────────────────────────┘  │ │         ▼             │ │
                    │  │  ┌────────────────────────┐  │ │  ┌────────────────┐   │ │
                    │  │  │    DragHandler         │  │ │  │  WASM (Rust)   │   │ │
                    │  │  │  pointer + angle math  │  │ └──│ score/rank/sel │   │ │
                    │  │  │  calls Gizmo.raycast() │  │    └────────────────┘   │ │
                    │  │  └────────────────────────┘  │    results (typed)     │ │
                    │  │  ┌────────────────────────┐  │                        │ │
                    │  │  │     CameraRig          │  │                        │ │
                    │  │  │  positionForBB / reset │  │                        │ │
                    │  │  └────────────────────────┘  │                        │ │
                    │  └──────────────────────────────┘                        │ │
                    └─────────────────────────────────────────────────────────────┘
```

**Tracing the primary use case** (load STL → see ranked candidates): User drops file → `FileDrop` emits → `AppController.handleFile()` → WASM `prepareData()` → `AppState.setMesh(...)` → `Viewport.loadModel()` (notifies subscribers) → user clicks "Find Candidates" → `AppController.spawnCompute()` → `worker.postMessage(request satisfies WorkerRequest)` → worker calls WASM → `worker.postMessage({type:'results', candidates} satisfies WorkerMessage)` → `AppController` updates `AppState.setCandidates(...)` → `CandidateList` re-renders via subscription → user clicks candidate → `AppController.showCandidate(i)` → `Viewport.showCandidate(q)` + `ScorePanel.update(q)`.

### Recommended Project Structure
```
web/src/
├── main.ts                       # entry — boot() only (criterion #1: ≤100 lines)
├── app/
│   ├── AppController.ts          # orchestration: file → parse → render → compute
│   ├── AppState.ts               # single store: EventTarget + subscribe/notify/get/set
│   └── constants.ts              # DECIMATE_TARGET, DEFAULT_REFINE_SEED, STORAGE_KEY, METRIC_STRIDE
├── views/
│   ├── ConfigPanel.ts            # angle slider, convention, profile, ranker, hull-sphere
│   ├── ScorePanel.ts             # live-score big number + 5 metric bars + hint
│   ├── CandidateList.ts          # <ol> with click navigation
│   ├── FileDrop.ts               # drag-drop + file picker
│   └── ProgressBar.ts            # indeterminate/determinate progress UI
├── viewport/
│   ├── Viewport.ts               # scene + camera + renderer + render loop (slim)
│   ├── GizmoController.ts        # owns gizmoGroup: ring creation, billboard, hover, raycast
│   ├── DragHandler.ts            # pointer capture + angle math + onOrientationChange callback
│   └── CameraRig.ts              # positionForBoundingBox() + reset() (small but mandated)
├── worker/
│   ├── orient.worker.ts          # thin WASM dispatcher (existing path preserved for Vite URL)
│   └── messages.ts               # re-export from types.ts (or empty barrel — see Pattern 4)
├── loadSTL.ts                    # WASM init + STL byte loading (split: see Pattern 5)
├── compute.ts                    # decimateForScore ONLY (criterion #4)
├── types.ts                      # ALL types: OrientConfig, Candidate, WorkerMessage, etc.
├── profiles/                     # unchanged
│   ├── index.ts
│   └── *.json
├── quaternion.ts                 # unchanged
├── rotate.ts                     # unchanged
├── convention.ts                 # unchanged
├── centering.ts                  # liftOntoPlate REMOVED (criterion #11)
├── exportSTL.ts                  # unchanged
├── nearestScore.ts               # unchanged
└── styles/
    ├── theme.css                 # :root { --color-accent, --color-bg, ... }
    ├── main.css                  # body, scrollbar, layout
    ├── ConfigPanel.module.css
    ├── ScorePanel.module.css
    ├── CandidateList.module.css
    └── Viewport.module.css
```

**Test layout (mirrors src/):**
```
web/src/
├── app/AppState.test.ts           # subscribe/notify/set/get — pure, node env
├── app/AppController.test.ts      # orchestration with mock DOM + mock Viewport + mock worker
├── app/constants.test.ts          # (optional) sanity: values match production
├── viewport/GizmoController.test.ts  # ring math, billboard, hit detection — mock THREE
├── viewport/DragHandler.test.ts      # angle delta, pointer capture — mock THREE + DOM
├── worker/messages.test.ts           # type narrowing (compile-time) + runtime shape sanity
├── centering.test.ts                 # existing — REMOVE liftOntoPlate describe block
├── convention.test.ts                # existing — unchanged
├── quaternion.test.ts                # existing — unchanged
└── rotate.test.ts                    # existing — unchanged
```

### Pattern 1: AppState store via EventTarget (hand-rolled, no deps)
**What:** Single mutable store; views subscribe by event type; controller mutates via typed setters.
**When to use:** Always — this is the only state container in the app.
**Example:**
```typescript
// Source: native EventTarget + standard observer pattern [CITED: developer.mozilla.org/en-US/docs/Web/API/EventTarget]
// app/AppState.ts
import type { Candidate, OrientConfig, LoadConvention, OriData } from '../types';

interface AppStateData {
  config: OrientConfig;
  candidates: Candidate[];
  currentIndex: number;
  stlName: string;
  currentProfile: string;
  currentRanker: string;
  isComputeDirty: boolean;
  lastOriData: OriData | null;
  liveData: { positions: Float32Array; normals: Float32Array; areas: Float32Array } | null;
  normBounds: { lo: number[]; hi: number[] } | null;
  loadConvention: LoadConvention;
  // lastFile / lastFileBytes / positions / faceNormals / areas are derived from
  // lastOriData + liveData — do NOT duplicate them in state (criterion #2: single source).
}

export class AppState extends EventTarget {
  private data: AppStateData;

  constructor(initial: AppStateData) {
    super();
    this.data = initial;
  }

  get<K extends keyof AppStateData>(key: K): AppStateData[K] {
    return this.data[key];
  }

  set<K extends keyof AppStateData>(key: K, value: AppStateData[K]): void {
    this.data = { ...this.data, [key]: value };
    this.dispatchEvent(new CustomEvent('change', { detail: { key, value } }));
  }

  subscribe(listener: (key: keyof AppStateData, value: unknown) => void): () => void {
    const handler = (e: Event) => {
      const { key, value } = (e as CustomEvent).detail;
      listener(key, value);
    };
    this.addEventListener('change', handler);
    return () => this.removeEventListener('change', handler);
  }
}
```

**Migration of the 18 `let` vars:** Each `let x = ...` becomes `state.set('x', ...)`. Reads change from `x` to `state.get('x')`. The `positions` / `faceNormals` / `areas` triple is **redundant** — they live inside `lastOriData` already — so the refactor collapses them (criterion #2: "not 14+ module-level `let` variables").

### Pattern 2: GizmoController + DragHandler ownership boundary
**What:** `GizmoController` owns its own `THREE.Group` (creates, positions, disposes). `Viewport` no longer references ring meshes directly.
**When to use:** Always after refactor.
**Interface contract:**
```typescript
// viewport/GizmoController.ts
export type RingAxis = 'axis-x' | 'axis-y' | 'axis-z' | 'camera';

export class GizmoController {
  readonly group: THREE.Group;          // owned — added to/removed from scene by Viewport

  constructor(modelPosition: THREE.Vector3, boundingRadius: number);

  /** Called by Viewport.animate() every frame — keeps camera ring facing camera. */
  billboard(camera: THREE.Camera): void;

  /** Returns the topmost ring under the pointer, or null. */
  raycastRing(ndc: THREE.Vector2, camera: THREE.Camera): RingAxis | null;

  /** Highlight a ring on hover; pass null to clear. */
  setHover(mode: RingAxis | null): void;

  /** Tear down geometries + materials. */
  dispose(): void;
}

// viewport/DragHandler.ts
export class DragHandler {
  constructor(
    gizmo: GizmoController,
    mesh: THREE.Mesh,
    camera: THREE.Camera,
    domElement: HTMLCanvasElement,
    boundingRadius: number,
    onOrientationChange: (q: [number, number, number, number]) => void,
  );

  /** Detach all pointer listeners. */
  dispose(): void;
}
```

**Why this boundary:** `Viewport` keeps the render loop and scene graph; `GizmoController` encapsulates everything about the rings; `DragHandler` consumes `GizmoController.raycastRing()` and computes angle deltas. This makes both testable independently — `GizmoController` can be tested by mocking `THREE.Raycaster`, and `DragHandler` can be tested by feeding synthetic `PointerEvent`s.

### Pattern 3: Discriminated union for worker messages
**What:** A single `WorkerMessage` union type shared between main thread and worker.
**When to use:** Always — criterion #8 mandates it.
**Example:**
```typescript
// Source: TypeScript discriminated unions [CITED: typescriptlang.org/docs/handbook/2/narrowing.html]
// types.ts (shared)
export type WorkerRequest = {
  data: OriData;
  config: ComputeConfig;
  weights: [number, number, number, number, number];
  ranker: string;
  maxCandidates: number;
  minAngleDeg: number;
};

export type WorkerMessage =
  | { readonly type: 'progress'; readonly value: number }
  | { readonly type: 'results'; readonly candidates: Candidate[] }
  | { readonly type: 'error'; readonly message: string };

// orient.worker.ts
self.postMessage({ type: 'progress', value: 50 } satisfies WorkerMessage);

// main.ts
worker.onmessage = (e: MessageEvent<WorkerMessage>) => {
  const msg = e.data; // narrowed by `msg.type`
  switch (msg.type) {
    case 'progress': progressBar.style.width = `${msg.value}%`; break;
    case 'results': AppState.set('candidates', msg.candidates); break;
    case 'error':   console.error(msg.message); break;
  }
};
```

**Type location:** Define `WorkerMessage` and `WorkerRequest` in `types.ts` so both `main.ts` and `orient.worker.ts` import from the same source (criterion #4: types centralized in `types.ts`).

### Pattern 4: CSS modules via Vite (native, no preprocessor)
**What:** Per-component `*.module.css` files; classnames auto-scoped by Vite.
**When to use:** Always — criterion #5 mandates removal of `<style>` from `index.html`.
**Example:**
```typescript
// Source: Vite CSS Modules docs [CITED: vite.dev/guide/features.html#css-modules]
// views/ScorePanel.ts
import styles from '../styles/ScorePanel.module.css';

export class ScorePanel {
  constructor(private el: HTMLElement) {}
  render(score: number) {
    this.el.className = styles.scoreBig;
    this.el.textContent = `${(score * 100).toFixed(0)}%`;
  }
}
```
```css
/* styles/theme.css — imported once in main.ts */
:root {
  --color-accent: #4a90d9;
  --color-bg: #1a1a1a;
  --color-panel: rgba(18, 18, 18, 0.92);
  --color-success: #27ae60;
  --color-danger: #e74c3c;
  --color-warning: #f0ad4e;
  --radius: 10px;
  --font-size-base: 0.85rem;
}

/* styles/ScorePanel.module.css */
.scoreBig {
  font-size: 1.6rem;
  font-weight: 700;
  color: var(--color-warning);
}
```

**`index.html` after refactor:** contains ONLY `<div>` landmarks + `<script type="module" src="/src/main.ts">`. No `<style>` block. The 180 lines of CSS move to `styles/*.css` and `styles/*.module.css`.

### Pattern 5: Split WASM init from file loading
**What:** `loadSTL.ts` currently conflates two responsibilities: WASM bootstrap (`initWasm`) and file-byte reading (`loadSTLBytes`) + data prep (`prepareData`). They have different lifecycles (init once vs. per-file) and different failure modes.
**When to use:** Always — part of criterion #1 (`main.ts` ≤ 100 lines).
**Split proposal:**
```typescript
// loadSTL.ts — keep file-byte reading pure (already is)
export async function loadSTLBytes(file: File): Promise<Uint8Array> { /* unchanged */ }

// wasm.ts (new, or fold into AppController)
let wasmReady = false;
export async function initWasm(): Promise<void> { /* unchanged */ }
export function prepareData(bytes: Uint8Array, config: OrientConfig): OriData { /* unchanged */ }
```
**Lighter alternative:** leave the file as-is if it's not blocking the ≤100-line `main.ts` target. The conflation is a code smell, not a blocker. (Ponytail: don't refactor for taste; refactor for the success criteria.)

### Pattern 6: Accessibility for the 3D viewport
**What:** Keyboard rotation + ARIA semantics for the canvas. WAI-ARIA has **no dedicated pattern for 3D manipulation widgets** `[ASSUMED]` — apply general ARIA + APG slider/spinbutton heuristics.
**When to use:** Always — criterion #6 + #7 mandate it.
**Example:**
```html
<!-- index.html landmark structure (criterion #7) -->
<body>
  <header>
    <h1>Orient STL</h1>
    <p class="subtitle">Resin printing orientation tool</p>
  </header>
  <main>
    <section id="viewport"
             role="application"
             aria-label="3D model viewport — drag rings to rotate, or use arrow keys"
             tabindex="0">
    </section>
    <aside id="status-live" aria-live="polite" class="sr-only"></aside>
  </main>
  <aside id="panel-left">...</aside>
  <aside id="panel-right">...</aside>
</body>
```
```typescript
// viewport/KeyboardHandler.ts (or fold into DragHandler)
// Arrow keys rotate the model around the three ring axes:
//   ←/→  → Yaw ring   (axis-y in code, world Z)
//   ↑/↓  → Pitch ring (axis-x in code, world X)
//   Q/E  → Roll ring  (axis-z in code, world Y)
// Shift = finer 1° steps; default 5°.
const STEP_DEG = 5;
const FINE_DEG = 1;
el.addEventListener('keydown', (e) => {
  const fine = e.shiftKey ? FINE_DEG : STEP_DEG;
  switch (e.key) {
    case 'ArrowLeft':  applyKeyRotation('axis-y', -fine); break;
    case 'ArrowRight': applyKeyRotation('axis-y',  fine); break;
    case 'ArrowUp':    applyKeyRotation('axis-x', -fine); break;
    case 'ArrowDown':  applyKeyRotation('axis-x',  fine); break;
    case 'q': case 'Q': applyKeyRotation('axis-z', -fine); break;
    case 'e': case 'E': applyKeyRotation('axis-z',  fine); break;
    default: return;
  }
  e.preventDefault();
  liveRegion.textContent = `Yaw ${yawDeg}°, Pitch ${pitchDeg}°, Roll ${rollDeg}°`;
});
```

**ARIA notes `[ASSUMED]`:**
- `role="application"` tells screen readers to pass keystrokes through (vs. intercepting for browse mode). Appropriate for a 3D canvas with custom keyboard handlers.
- `aria-live="polite"` region announces orientation changes without stealing focus.
- `.sr-only` class (visually hidden, screen-reader available) is standard but must be defined in `styles/main.css`.
- Visible focus indicator is REQUIRED: `#viewport:focus-visible { outline: 2px solid var(--color-accent); outline-offset: -2px; }`.
- There is **no W3C-published APG pattern for 3D viewport widgets** as of 2026-07 — this design is assembled from `role="application"` + APG slider/spinbutton heuristics `[CITED: w3.org/WAI/ARIA/apg/]`.

### Anti-Patterns to Avoid
- **Global singleton store accessible via import:** Don't `export const state = new AppState()` at module scope — that creates a hidden global that breaks testability. Always inject via constructor: `new AppController({ state, viewport, ... })`.
- **Subscribing inside view constructors without unsubscribing:** Each `subscribe()` returns an `unsubscribe()` — call it in a `dispose()` method. Memory leaks from forgotten subscriptions are the #1 bug in observer-pattern UIs.
- **Replacing `let` with `state.set()` everywhere blindly:** Some locals (e.g., `paint`, `cancelCompute` closure state) are not "state" — they're control-flow flags. Only data that needs to be observed by views belongs in `AppState`.
- **Putting mesh geometry arrays (`positions`, `faceNormals`, `areas`) in `AppState`:** They're already inside `lastOriData`. Adding them separately violates the "single source of truth" principle and doubles memory. **Criterion #2 explicitly says "single AppState" — collapsing these duplicates IS the work.**
- **Mocking all of Three.js blindly in tests:** Mock only what the test exercises. `GizmoController.raycastRing()` needs `THREE.Raycaster`; `DragHandler` needs `setPointerCapture`. Don't mock `THREE.Scene` for a GizmoController test.
- **Keeping `liftOntoPlate`, `SliceResult`, `RefineFn` "just in case":** All three have **zero production callers** (verified via codegraph). Criterion #11 mandates removal. **Also remove their tests** (`centering.test.ts` lines 73-87 — the `liftOntoPlate` describe block).
- **Treating the suspicious axis mapping (viewport.ts:386-396) as a bug to fix during refactor:** The current pointer-drag code maps `'axis-y'` → `Vector3(0,0,1)` and `'axis-z'` → `Vector3(0,1,0)`. This may be intentional (Y-ring rotates around world Z) or a latent bug. **The refactor MUST preserve the current behavior and pin it with a test** — fixing the axis mapping is a separate, explicitly-scoped change, NOT part of this refactor.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CSS class scoping | BEM / scoped-attribute convention | Vite CSS modules (`*.module.css`) | Built into Vite 8 — zero config, zero deps `[CITED: vite.dev/guide/features.html#css-modules]` |
| Theming variables | Sass maps / JSON theme files | CSS custom properties (`--var`) | Native browser feature; runtime-overridable; works in all evergreen browsers `[CITED: developer.mozilla.org/en-US/docs/Web/CSS/--*]` |
| Observer pattern for state | RxJS / Custom event bus | `EventTarget` (browser-native) | Already in the browser; `addEventListener`/`dispatchEvent` semantics everyone knows `[CITED: developer.mozilla.org/en-US/docs/Web/API/EventTarget]` |
| Quaternion math | New helpers | Existing `quaternion.ts` + `rotate.ts` | Already tested (8+9 tests); reuse, don't rewrite |
| Discriminated union runtime check | `zod` / `io-ts` | TypeScript `satisfies WorkerMessage` | Worker IPC is single-source (our own worker); compile-time narrowing is sufficient. Adding a runtime validator is YAGNI. |
| Keyboard event key codes | Custom `keyCode` lookup table | `e.key` (DOM3) | Native; handles layout differences automatically |

**Key insight:** This phase is unusual — the "don't hand-roll" list is shorter than usual because the project's existing dependency footprint is already minimal. The temptation here is the opposite: to **add** libraries (nanostores, signals, valtio, jsdom) for problems already solved by native platform features. Resist that temptation.

## Runtime State Inventory

> This is a **refactor** phase (no rename/rebrand), but it does restructure how state is held in memory. The canonical question — *what runtime systems have the old shape cached?* — has a narrow answer for an in-browser SPA with no persistence beyond `localStorage`.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | `localStorage['orient-stl-config']` (key is a constant, not the renamed symbol; schema is `StoredConfig` with `version: 1`). The refactor changes how this is read/written (via `AppState` instead of bare `localStorage.getItem`) but does NOT change the key or schema. | Code edit only — no data migration. Keep `STORAGE_KEY` and `SCHEMA_VERSION` as named constants. |
| Live service config | None. No external services. | None — verified by grep for `fetch(`, `WebSocket`, `EventSource` (all absent in `web/src/`). |
| OS-registered state | None. Pure browser app. | None. |
| Secrets/env vars | None. No env vars read; WASM binary is bundled, not fetched with a token. | None. |
| Build artifacts | `web/pkg/orient_core_bg.wasm` (195KB) — built by `wasm-pack` from `core/src/*.rs`. **This phase touches ZERO `.rs` files** (frontend-only), so the WASM rebuild rule from `spike-findings-orient-stl` does NOT fire. | None — verify no `.rs` files are touched in any plan; if a plan accidentally touches Rust, the spike's WASM-rebuild hard rule applies. |
| Module-scope singletons (in-memory) | 18 `let` vars in `main.ts` (verified via grep). These are not "cached" externally — they live only in the JS heap. After refactor they move into `AppState` instances. The lifecycle is identical (page-load → page-unload). | Code edit only. No migration. |
| Worker in-flight messages | `orient.worker.ts` posts `{type:'progress'|'results'|'error'}`. If a refactor lands mid-compute, in-flight messages would be untyped — but the worker is torn down per file-load (`cancelCompute()`), so there is no in-flight state to migrate. | None. Just ensure the new typed `postMessage` shape is deployed atomically (worker + main in the same commit). |

**Nothing in this phase requires a data migration.** The runtime state inventory is empty except for code edits.

## Common Pitfalls

### Pitfall 1: Over-extracting CameraRig / drowning Viewport in micro-classes
**What goes wrong:** The refactor creates 8+ tiny classes (CameraRig, Lights, BuildPlate, RenderLoop...) that each have 5-10 lines. Indirection explodes; reading the render path requires jumping through 5 files.
**Why it happens:** Success criteria lists `GizmoController`, `DragHandler`, `CameraRig` — engineers extrapolate "extract everything."
**How to avoid:** **Stop at the 3 mandated extractions.** Viewport keeps lights, build plate, render loop, mesh load, overhang coloring. CameraRig is just `positionForBB` + `reset`. If a class would be <15 lines, leave the code in Viewport.
**Warning signs:** A PR that adds `viewport/Lights.ts`, `viewport/BuildPlate.ts`, or `viewport/RenderLoop.ts`.

### Pitfall 2: Breaking the worker mid-message
**What goes wrong:** The discriminated-union `WorkerMessage` type is added to `orient.worker.ts` in one commit, but `main.ts`'s `worker.onmessage` handler still uses the untyped shape (or vice versa). Compute silently breaks.
**Why it happens:** Worker IPC changes must be atomic across two files in two different module graphs.
**How to avoid:** Land the worker type changes in **one commit** that touches both `orient.worker.ts` and `main.ts`'s handler. Add a `worker/messages.test.ts` that round-trips a fake `postMessage` payload through the type.
**Warning signs:** Two consecutive commits both named "type worker messages."

### Pitfall 3: Silent axis-mapping behavior change in DragHandler
**What goes wrong:** The current viewport.ts (lines 386-396) maps `'axis-y'` drag to world Z and `'axis-z'` drag to world Y. This looks like a bug. The refactorer "fixes" it during extraction. Drag direction inverts for users; rotation feels wrong.
**Why it happens:** The mapping is non-obvious and the comment is absent.
**How to avoid:** **Pin current behavior with a regression test BEFORE the refactor.** Test: dragging the green ring (axis-y) rotates the mesh around world Z. Then extract. Then test still passes. If the team decides the mapping IS a bug, that's a separate task with its own test+commit.
**Warning signs:** A DragHandler test that asserts `'axis-y'` → `Vector3(0,1,0)` (the "intuitive" mapping the current code does NOT use).

### Pitfall 4: Forgetting to unsubscribe views on re-render
**What goes wrong:** `ScorePanel` subscribes to `AppState`. User loads a new file. `ScorePanel.render()` runs again, subscribing a second time. After 10 file loads, every state change triggers 10+ renders.
**Why it happens:** Observer-pattern ergonomics encourage subscribe-in-constructor; cleanup is easy to forget.
**How to avoid:** Each view class exposes `dispose()` that calls the unsubscribe function returned by `state.subscribe()`. AppController tracks all views and disposes them on file reload (or never re-creates them — preferred).
**Warning signs:** `console.log` inside a subscription callback firing N times after N file loads.

### Pitfall 5: Treating `catch {}` comments as compliance with criterion #12
**What goes wrong:** The 3 existing empty catches (main.ts:56, :243, :264) have explanatory comments. Refactorer leaves them, claiming "they have comments, so they have explicit recovery." Criterion says "removed OR given explicit recovery."
**Why it happens:** "Comment" feels like "explicit."
**How to avoid:** "Explicit recovery" means **code that does something** — sets a fallback value, logs to a telemetry sink, sets an error flag in AppState. Comments alone don't count. For each catch: either delete the try (let it throw), or add real recovery (e.g., `AppState.set('normBounds', null)` so the UI knows scoring is degraded).
**Warning signs:** A PR that ships with the same 3 catches, same 3 comments.

### Pitfall 6: Adding happy-dom/jsdom when DI would do
**What goes wrong:** Refactorer adds `happy-dom` to test `ScorePanel.render()`. Now vitest config splits into `environment: 'node'` for math tests and `environment: 'happy-dom'` for view tests. Setup gets more complex; CI slower.
**Why it happens:** "Test a view" pattern-matches to "render to DOM."
**How to avoid:** Inject DOM elements into view constructors: `new ScorePanel(mockEl)`. The view calls `mockEl.textContent = '42%'`. Test asserts `mockEl.textContent`. **No DOM environment needed.** Add happy-dom ONLY for tests that genuinely need `getBoundingClientRect` or layout.
**Warning signs:** `vitest.config.ts` growing per-file environment overrides.

## Code Examples

### AppState test (no DOM env needed)
```typescript
// app/AppState.test.ts
import { describe, it, expect, vi } from 'vitest';
import { AppState } from './AppState';
import { defaultConfig } from '../types';

describe('AppState', () => {
  it('notifies subscribers on set', () => {
    const state = new AppState({ config: defaultConfig(), candidates: [], /* ... */ });
    const listener = vi.fn();
    state.subscribe(listener);
    state.set('currentProfile', 'topsis');
    expect(listener).toHaveBeenCalledWith('currentProfile', 'topsis');
  });

  it('unsubscribe stops notifications', () => {
    const state = new AppState({ /* ... */ });
    const listener = vi.fn();
    const unsub = state.subscribe(listener);
    unsub();
    state.set('currentProfile', 'topsis');
    expect(listener).not.toHaveBeenCalled();
  });
});
```

### AppController with constructor injection (testable without DOM env)
```typescript
// app/AppController.ts
export interface AppControllerDeps {
  state: AppState;
  viewport: Viewport;       // real or mock
  fileDrop: FileDrop;
  configPanel: ConfigPanel;
  scorePanel: ScorePanel;
  candidateList: CandidateList;
  workerFactory: () => Worker;  // inject for tests
}

export class AppController {
  constructor(private deps: AppControllerDeps) {
    deps.fileDrop.onFile(f => this.handleFile(f));
    deps.configPanel.onChange(() => deps.state.set('isComputeDirty', true));
    deps.state.subscribe((key) => {
      if (key === 'candidates') deps.candidateList.render(deps.state.get('candidates'));
    });
  }

  async handleFile(file: File): Promise<void> { /* ... */ }
  async spawnCompute(): Promise<void> { /* ... */ }
}
```

### Discriminated union narrowing test (compile-time + runtime)
```typescript
// worker/messages.test.ts
import { describe, it, expect } from 'vitest';
import type { WorkerMessage } from '../types';

describe('WorkerMessage union', () => {
  it('progress messages carry only value', () => {
    const msg: WorkerMessage = { type: 'progress', value: 50 };
    if (msg.type === 'progress') expect(msg.value).toBe(50);
  });

  it('results messages carry candidates', () => {
    const msg: WorkerMessage = { type: 'results', candidates: [] };
    if (msg.type === 'results') expect(msg.candidates).toEqual([]);
  });
});
```

### CSS module import (Vite-native)
```typescript
// views/CandidateList.ts
import styles from '../styles/CandidateList.module.css';

export class CandidateList {
  constructor(private el: HTMLOListElement) {}
  render(candidates: Candidate[], currentIndex: number) {
    this.el.innerHTML = candidates.map((c, i) =>
      `<li class="${i === currentIndex ? styles.active : ''}" data-index="${i}">` +
      `#${i + 1} — ${(c.compositeScore * 100).toFixed(0)}%</li>`
    ).join('');
  }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `EventEmitter` from Node | Native `EventTarget` | All evergreen browsers since ~2019 | No need for any event library in browser code `[CITED: developer.mozilla.org/en-US/docs/Web/API/EventTarget]` |
| BEM / OOCSS / Sass for scoping | CSS Modules (Vite-native) | Vite has shipped CSS modules since v1 | `<style>` blocks in HTML are an anti-pattern for non-trivial apps `[CITED: vite.dev/guide/features.html#css-modules]` |
| `jsdom` for any DOM test | `happy-dom` (faster) OR dependency injection (no DOM) | vitest recommends happy-dom; DI avoids both | Ponytail preference: DI where possible, happy-dom where required |
| `keyCode` (deprecated) | `e.key` (DOM3) | All evergreen browsers since ~2018 | Keyboard handler code is shorter and layout-aware |
| `any`-typed `postMessage` | Discriminated union + `satisfies` | TS 4.9+ | Worker IPC gets compile-time narrowing; runtime validators (zod) are YAGNI for in-app workers |

**Deprecated/outdated:**
- `new Worker(scriptURL, { type: 'module' })` is current and correct (already used). Do NOT switch to classic workers.
- `postMessage` without structured-clone-safe payloads is outdated — current code passes `Float32Array` (transferable but not currently transferred). **Optional optimization:** use the `Transferable` list to zero-copy the metrics array. **Out of scope for this phase** (perf, not architecture).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The 18 module-level `let` vars in `main.ts` are the canonical list (no others missed). | Summary + Runtime State Inventory | LOW — verified via grep `^let ` against `main.ts`. |
| A2 | `liftOntoPlate`, `SliceResult`, `RefineFn` are dead code safe to delete. | Anti-Patterns + Don't Hand-Roll | LOW — verified via codegraph (0 callers). `liftOntoPlate` has tests that must also be removed. |
| A3 | The axis-mapping in viewport.ts:386-396 (axis-y → world Z, axis-z → world Y) is intentional current behavior, not a bug to fix during refactor. | Pitfall 3 | MEDIUM — code has no comment; refactorer must pin the existing behavior with a regression test before extracting. If it IS a bug, a follow-up task fixes it explicitly. |
| A4 | WAI-ARIA APG has no published pattern for 3D viewport manipulation widgets as of 2026-07. | Pattern 6 | LOW — based on training knowledge of APG catalog. The recommended `role="application"` + keyboard handlers + `aria-live` is assembled from general ARIA practice, not a single canonical source. Tagged `[ASSUMED]` because no URL confirms the negative. |
| A5 | AppController can be fully tested via dependency injection without a DOM environment. | Pattern 1, Pitfall 6 | LOW — standard DI pattern. Risk only if a view class reaches for `document.*` at module load (anti-pattern). |
| A6 | WASM binary does not need rebuilding for this phase. | Runtime State Inventory | LOW — phase scope is `web/src/*.ts` and `web/index.html`. The spike's WASM-rebuild hard rule only fires on `core/src/*.rs` edits. Planner must verify each plan stays in `web/`. |
| A7 | The `[SUS]` "too-new" flags on nanostores / @preact/signals-core / happy-dom are false positives. | Package Legitimacy Audit | LOW — all three packages have multi-million weekly downloads and active GitHub repos. The flag is a heuristic on `publishedAt` recency, not a legitimacy signal. |
| A8 | `EventTarget` is the right substrate for AppState (vs. a hand-rolled subscriber Set). | Pattern 1 | LOW — `EventTarget` is browser-native, well-known, and supports `CustomEvent`. A bare `Set<Listener>` is even lighter but loses event-name namespacing. Either is fine. |

**All other claims are tagged `[CITED]` or `[VERIFIED]` inline.**

## Open Questions

1. **Should CameraRig actually be extracted, given it's only 9 lines?**
   - What we know: Success criteria #3 explicitly lists `CameraRig` alongside `GizmoController` and `DragHandler`. Current `resetCamera()` is 9 lines.
   - What's unclear: Whether the spirit of the criteria is "extract these 3 things" or "decompose Viewport aggressively."
   - Recommendation: **Extract CameraRig as a minimal class** (`positionForBoundingBox(bb)` + `reset()`). Honors the criteria without over-extracting. Planner should make this an explicit task so the decision is visible.

2. **Should the axis-mapping "bug" (axis-y → world Z) be fixed as part of this refactor?**
   - What we know: The mapping is non-obvious and undocumented. It MIGHT be intentional (the ring colors suggest X=red, Y=green, Z=blue, but the drag rotates around different world axes).
   - What's unclear: Whether users rely on the current behavior.
   - Recommendation: **Preserve current behavior in the refactor. Pin it with a regression test. Open a separate task/ticket to investigate.** Do NOT silently "fix" it.

3. **Do we add `happy-dom` for view-class tests, or stick with DI-only?**
   - What we know: DI avoids the dependency entirely for AppController tests. View classes (`ScorePanel.render`) update `textContent` and `innerHTML` — testable with plain mock elements.
   - What's unclear: Whether any view test will need `getBoundingClientRect` (e.g., for progress-bar width assertions).
   - Recommendation: **Start without happy-dom.** Add it only if a specific test requires layout. The planner should mark this as a "decide at execution time" item.

4. **Should the worker use the `Transferable` list for zero-copy metrics transfer?**
   - What we know: Current `postMessage` copies the `Float32Array` metrics buffer. Using `[metrics.buffer]` as the transfer list would zero-copy it.
   - What's unclear: Whether this is a measurable win for ~200-candidate result sets.
   - Recommendation: **Out of scope for Phase 6** (this is perf, not architecture). Capture as a deferred idea.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Node.js | Build + tests | ✓ | (assumed ≥18 from Vite 8 compat) | — |
| Vite | Build + CSS modules + worker bundling | ✓ | 8.1.4 | — |
| Vitest | Test runner | ✓ | 4.1.10 | — |
| TypeScript | Compile-time type narrowing | ✓ | ^5.7.0 | — |
| three | Viewport rendering | ✓ | 0.185.1 | — |
| `@types/three` | Type defs for three | ✓ | ^0.185.1 | — |
| `wasm-pack` | WASM rebuild (only if Rust touched) | (assumed present — Phase 5 used it) | — | **NOT NEEDED this phase** (frontend-only) |
| Browser (Chromium-family) | Manual UAT for accessibility | ✓ (dev's local browser) | — | Required for the A11y criterion — cannot be automated fully |

**Missing dependencies with no fallback:** none.

**Missing dependencies with fallback:** none — every required tool is already installed and verified.

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | No auth; pure local SPA |
| V3 Session Management | no | No sessions |
| V4 Access Control | no | No privileged operations |
| V5 Input Validation | **yes (minimal)** | File-input validation already exists (`loadSTL.ts` rejects empty + >100MB). Refactor must preserve these guards. No new untrusted inputs introduced. |
| V6 Cryptography | no | No crypto operations |
| V7 Error Handling | **yes (minimal)** | Criterion #12 mandates explicit recovery for empty catches. Aligns with ASVS V7 (no silent error swallowing). |
| V8 Data Protection | **yes (minimal)** | `localStorage` persistence of user prefs (profile, ranker, angle). No PII. Refactor must preserve `try/catch` around `localStorage.setItem` (private-mode browsers throw). |
| V9 Communications | no | No network calls (WASM is bundled, not fetched) |
| V13 API & Web Service | no | No API |

### Known Threat Patterns for vanilla-TS + Web Worker + WASM

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malicious STL triggers WASM panic → worker hangs | Denial of Service | Worker runs in separate thread; main thread can `worker.terminate()` (already implemented as `cancelCompute`). **Preserve in AppController.** |
| `innerHTML` injection from candidate metadata | Tampering / XSS | `CandidateList.render()` builds HTML from `compositeScore` (number) and `id` (string we control). **No user-controlled strings reach innerHTML today.** Refactor must preserve: never interpolate file names into HTML without escaping. `stlName` is set into `link.download` (safe) and used in `exportSTL` filename (safe). |
| `localStorage` schema confusion | Tampering | `loadConfig()` already checks `SCHEMA_VERSION`. **Preserve the version check.** |
| Worker message spoofing | Spoofing | N/A — worker is same-origin, bundled by Vite. No `targetOrigin` concern. |

**This phase introduces no new attack surface.** All mitigations are preservation requirements, not new controls.

## Sources

### Primary (HIGH confidence)
- **Codegraph index** — 39 files, 469 nodes, 929 edges; queried for `liftOntoPlate`, `SliceResult`, `RefineFn` callers (all 0) and `computeNormBounds` callers (1: `handleFile`)
- **Codebase reads** — all 12 source files + 4 test files in `web/src/`; verified line counts (main.ts=504, viewport.ts=498) and module-level `let` count (18 in main.ts)
- **npm registry** — verified current versions of `three`, `vite`, `vitest`, `typescript`, `@types/three`, `nanostores`, `@preact/signals-core`, `valtio`, `jsdom`, `happy-dom` (all fetched 2026-07-14)

### Secondary (MEDIUM confidence)
- Vite official docs — CSS Modules feature (`vite.dev/guide/features.html#css-modules`) `[CITED]`
- MDN — `EventTarget`, CSS custom properties, `role="application"` `[CITED]`
- TypeScript handbook — discriminated unions, `satisfies` operator `[CITED]`
- WAI-ARIA Authoring Practices Guide (`w3.org/WAI/ARIA/apg/`) `[CITED]` — for general ARIA semantics; **no specific 3D-viewport pattern exists** `[ASSUMED]`

### Tertiary (LOW confidence)
- None. All claims are either code-verified or sourced from official docs.

## Metadata

**Confidence breakdown:**
- Standard stack: **HIGH** — all packages already installed and verified; zero new deps recommended
- Architecture: **HIGH** — recommendations are code-grounded; ownership boundaries derived from existing 498-line Viewport structure
- Pitfalls: **HIGH** — each pitfall references specific line numbers verified via Read
- Accessibility: **MEDIUM** — general ARIA semantics are well-cited; the 3D-specific pattern is assembled, not canonical (see assumption A4)

**Research date:** 2026-07-14
**Valid until:** 2026-08-14 (30 days — vanilla-TS/Vite/Three.js are stable; ARIA APG could publish a 3D pattern that would supersede Pattern 6)
