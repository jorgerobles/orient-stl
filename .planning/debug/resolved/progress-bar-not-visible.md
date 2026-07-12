---
status: diagnosed
trigger: "Progress bar not visible during sync phases (Reading, Parsing STL, Rendering, Decimating)"
created: "2026-07-11T00:00:00Z"
updated: "2026-07-11T00:00:00Z"
---

## Current Focus

hypothesis: "CONFIRMED — Root cause found (see Resolution)"
test: "Code analysis + timing measurements + MutationObserver + computed style analysis"
expecting: ""
next_action: "Return final diagnosis to user"

## Symptoms

expected: "Progress bar shows the indeterminate sliding animation during each phase (Reading, Parsing, Rendering, Decimating), and the label text updates visibly for each phase"
actual: "Progress bar shows no visible progress — either frozen or invisible during the entire processing pipeline, then disappears when results appear"
errors: ""
reproduction: "Load any .stl file into the web app"
started: "likely always broken"

## Eliminated

-

## Evidence

- timestamp: "2026-07-11T00:00:00Z"
  checked: "Code analysis — handleFile() and spawnCompute() execution flow in web/src/main.ts"
  found: "handleFile() sets progress bar DOM (display, label, className) then hits `await loadSTLBytes(file)`. After await resolves, ALL remaining work is synchronous (prepareData WASM, Float32Array constructors, viewport.loadModel, viewport.resetCamera, decimateForScore). spawnCompute then creates segments and spawns Web Workers (async)."
  implication: "The `await` yields to event loop, but only once. After that, the main thread is blocked by synchronous processing until workers are spawned."

- timestamp: "2026-07-11T00:00:00Z"
  checked: "Timing measurements via console.time — small STL (284 bytes, tetrahedron)"
  found: "reading file (await loadSTLBytes): 182ms | parsing STL (prepareData WASM): 31ms | rendering (loadModel + resetCamera): 15ms | decimating (decimateForScore): 0.67ms | total: 245ms"
  implication: "The async await takes 182ms. After that, 47ms of synchronous CPU work. The browser CAN paint the initial progress bar during the 182ms await, but the 47ms of sync work freezes it."

- timestamp: "2026-07-11T00:00:00Z"
  checked: "Timing measurements — large STL (1.6MB sphere, 5000 triangles)"
  found: "reading file (await loadSTLBytes): 271ms. Then prepareData WASM, loadModel, and decimateForScore run synchronously — no timing output captured before the page crashed (likely due to lengthy Three.js processing for 5000 triangles)."
  implication: "For larger files, the synchronous processing can take many seconds, completely freezing the main thread and the progress bar CSS animation."

- timestamp: "2026-07-11T00:00:00Z"
  checked: "MutationObserver logs — both small and large STL"
  found: "Mutations observed in order: [1] progress-container display -> block [2] progress-bar className -> progress-bar-fill indeterminate [3] progress-label text -> 'Reading file...' [4] (no mutation for 'Parsing STL (WASM)...') [5] (no mutation for 'Rendering model...') [6] (no mutation for 'Decimating mesh...') [7] progress-label text -> 'Computing candidates...' [8] progress-container display -> none"
  implication: "Only two label states are EVER observed by the DOM: 'Reading file...' and 'Computing candidates...'. The intermediate labels ('Parsing STL (WASM)...', 'Rendering model...', 'Decimating mesh...') are SET on the DOM but overwritten by the next label change before the browser's rendering pipeline runs. MutationObserver batches them — only the terminal value is dispatched."

- timestamp: "2026-07-11T00:00:00Z"
  checked: "Computed CSS animation state on progress bar (error case — bar frozen with indeterminate class)"
  found: "barAnimationName: 'indeterminate' | barComputedWidth: '35%' | barClassName: 'progress-bar-fill indeterminate'"
  implication: "The CSS animation IS correctly configured and the class IS applied. The animation would run if the browser's rendering pipeline had a chance to start it. But since the main thread is immediately blocked by synchronous work after the await resolves, the animation never progresses."

- timestamp: "2026-07-11T00:00:00Z"
  checked: "CSS animation property analysis"
  found: "The indeterminate animation uses `left` property: `@keyframes indeterminate { 0% { left: -40%; } 100% { left: 110%; } }`. The `left` property with `position: relative` requires layout recalculation — it is NOT a compositor-only property (unlike `transform` or `opacity`)."
  implication: "Even if the animation started, it would freeze during main-thread blocking because `left` animation needs the main thread to recalculate layout at each frame. Using `transform: translateX()` instead would allow the animation to run on the compositor thread independently."

- timestamp: "2026-07-11T00:00:00Z"
  checked: "Browser event loop analysis — microtask timing"
  found: "`await loadSTLBytes(file)` resolves via microtask. The microtask queue is processed BEFORE the browser's rendering step. So when `file.arrayBuffer()` resolves, the async function's continuation (prepareData → loadModel → decimateForScore) runs as a microtask chain that completes before the next rendering step. All DOM mutations during this chain are batched and painted atomically when it finishes."
  implication: "The browser NEVER renders intermediate DOM states between the await resolution and the completion of all synchronous work. Only the final state ('Computing candidates...' + segments or hidden) is ever painted."

## Resolution

root_cause: "Three interacting causes: (1) After the sole `await` in `handleFile()` resolves, all processing (WASM parse, Three.js render, mesh decimation) runs synchronously on the main thread via microtask continuation, preventing the browser from painting any intermediate DOM updates. (2) The CSS indeterminate animation uses `left` (layout-dependent property) which freezes when the main thread is blocked. (3) Intermediate progress labels ('Parsing STL...', 'Rendering model...', 'Decimating mesh...') are overwritten before the browser paints a frame that shows them."
fix: ""
verification: ""
files_changed: []
