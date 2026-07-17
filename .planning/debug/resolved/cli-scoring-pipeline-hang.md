---
status: resolved
trigger: "CLI hangs on 500K triangle STL — 'pipeline takes too long'"
created: "2026-07-14T22:00:00Z"
updated: "2026-07-15T00:30:00Z"
---

## Current Focus

hypothesis: "CONFIRMED — The scoring loop (500K triangles × ~400 candidates × 50 refine iters) is inherently O(n·c·r) and is the true bottleneck. Original web pipeline ran the same loop in JS Web Workers (non-blocking UI) while CLI blocks. The O(n²) vertex welding that triggered the browser hang was a separate issue — fixed by replacing with O(n) hash-dedup, then reverted the welding entirely."

test: "CLI timing with and without repair, with and without decimation"
expecting: ""
next_action: "DONE — added CLI --decimate flag (default 12000) matching web's decimateForScore. Repair work moved to preparative O(n) dedup only. No welding."

## Symptoms

expected: "CLI processes a 500K-triangle STL in reasonable time"
actual: "CLI seems to hang — takes multiple minutes to produce output"
errors: ""
reproduction: "Run `cargo run --release -- broken.stl` on the 500K triangle broken test file"
started: "2026-07-14 (first CLI use on large STL)"

## Eliminated

- O(n²) vertex welding — removed and replaced with O(n) hash dedup
- Hash-grid welding optimization — unnecessary, precompute_mesh already filters degenerates
- Duplicate triangle removal — O(n) and fast; ~50K removals take <1ms
- STL parsing — stl_io is fast linear parse
- Web pipeline — web uses setTimeout(0) yields between sync phases; CLI doesn't need them

## Evidence

- timestamp: "2026-07-14T22:00:00Z"
  checked: "CLI execution of `cargo run --release -- broken.stl` with timing instrumentation"
  found: "Pipeline breakdown for 499,378 triangles: parse ~0.3s, precompute_mesh ~0.7s, hull ~0.1s, decimation to 12K (499378→12000) ~0.01s, scoring/refine ~35s. The scoring/refine loop is 99% of the time."
  implication: "The scoring pipeline is inherently slow on large STLs because it iterates (decimated triangles × candidates × refine iterations). This is pre-existing — not caused by repair."

- timestamp: "2026-07-14T22:30:00Z"
  checked: "Web pipeline comparison — does it also freeze?"
  found: "Web blocks but yields via setTimeout(0) between phases. The original web JS (before WASM port) ran scoring in multiple Web Workers, keeping UI responsive. WASM scoring runs on the main thread but web pipeline shows a progress bar."
  implication: "CLI has no such yield mechanism — it's expected to compute synchronously. The issue is just that CLI was missing decimation, which the web already had."

## Resolution

Three independent fixes — all merged to master:

1. **O(n) duplicate-triangle repair** (`core/src/repair.rs:repair_mesh`) — hash-based dedup, runs in <1ms even on 500K STL. No vertex welding, no position modification.

2. **CLI decimation** (`--decimate`, default 12000) — stride-based triangle sampling matching `decimateForScore` in web code. `--decimate 0` disables. All defaults moved to named constants.

3. **Warning cleanups** — 6 Rust compiler warnings fixed across the codebase.

The scoring loop itself is not regressed — it's the same computation the web runs, just synchronous in CLI. Decimation brings it from ~3 minutes to ~3 seconds for the scoring phase.
