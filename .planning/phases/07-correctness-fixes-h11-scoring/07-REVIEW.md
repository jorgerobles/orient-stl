---
phase: 07-correctness-fixes-h11-scoring
reviewed: 2026-07-15T12:00:00Z
depth: standard
files_reviewed: 19
files_reviewed_list:
  - core/src/candidates.rs
  - core/src/harness.rs
  - core/src/lib.rs
  - core/src/main.rs
  - core/src/ranking.rs
  - core/src/scoring.rs
  - core/src/stability.rs
  - web/src/app/AppController.ts
  - web/src/profiles/cross-only.json
  - web/src/profiles/equal.json
  - web/src/profiles/footprint-only.json
  - web/src/profiles/height-only.json
  - web/src/profiles/overhang-footprint.json
  - web/src/profiles/overhang-only.json
  - web/src/profiles/resin-biased.json
  - web/src/profiles/surface-only.json
  - web/src/types.ts
  - web/src/views/ScorePanel.test.ts
  - web/src/views/ScorePanel.ts
findings:
  critical: 3
  warning: 5
  info: 2
  total: 10
status: issues_found
---

# Phase 07: Code Review Report

**Reviewed:** 2026-07-15T12:00:00Z
**Depth:** standard
**Files Reviewed:** 19
**Status:** issues_found

## Summary

Reviewed the H11 scoring phase covering 6-metric scoring components (overhang, footprint, cross-section, surface quality, height risk, shadowed overhang), three ranking algorithms (weighted-sum, consensus, TOPSIS), stability analysis, CLI pipeline, and web ScorePanel. Found 3 critical issues: CLI compilation failure due to missing `w_shadowed` field, a quaternion layout mismatch between WASM output and web consumer that produces wrong visual orientations and STL exports, and an incorrect docstring that masks the quaternion bug. Several warnings around stale documentation, redundant computation, and a subtle `--with-identity --all-rankings` data truncation bug.

## Critical Issues

### CR-01: CLI `main.rs` will not compile — `ScoreWeights` struct literal missing `w_shadowed`

**File:** `core/src/main.rs:282,286-289,339-345`
**Issue:** The `ScoreWeights` struct (ranking.rs:9-16) has 6 fields including `w_shadowed`, but `main.rs` constructs `ScoreWeights` in 3 places with only 5 fields. Rust requires all struct fields in a literal (no defaults). The CLI feature flag build will fail with "missing field `w_shadowed` in initializer of `ScoreWeights`".

Affected lines:
- **Line 282** (`all_rankings` path): `ScoreWeights { w_overhang: 1.0, w_footprint: 1.0, w_cross: 1.0, w_surface: 1.0, w_height: 1.0 };`
- **Lines 286-289** (profile loop): `ScoreWeights { w_overhang: pw[0], w_footprint: pw[1], w_cross: pw[2], w_surface: pw[3], w_height: pw[4] };`
- **Lines 339-345** (single-method path): `ScoreWeights { w_overhang: args.weights[0], ... w_height: args.weights[4] };`

Additionally, the `PROFILES` const (line 104) and `parse_weights` (line 90) only define/parse 5 weights, so even after fixing compilation, the CLI cannot express a shadowed weight.

**Fix:**
Add `w_shadowed: 0.0` (or a 6th CLI weight) to all 3 struct literals, extend `PROFILES` to `[f32; 6]`, and update `parse_weights` to accept 6 comma-separated values:

```rust
// Line 282
let primary_w = ScoreWeights { w_overhang: 1.0, w_footprint: 1.0, w_cross: 1.0,
    w_surface: 1.0, w_height: 1.0, w_shadowed: 1.0 };

// Lines 286-289
let w = ScoreWeights {
    w_overhang: pw[0], w_footprint: pw[1], w_cross: pw[2],
    w_surface: pw[3], w_height: pw[4], w_shadowed: pw[5],
};

// Lines 339-345
let w = ScoreWeights {
    w_overhang: args.weights[0], w_footprint: args.weights[1],
    w_cross: args.weights[2], w_surface: args.weights[3],
    w_height: args.weights[4], w_shadowed: args.weights[5],
};
```

### CR-02: Quaternion layout mismatch — WASM outputs `[w,x,y,z]`, web consumes as `[x,y,z,w]`

**File:** `core/src/lib.rs:249,284-285`
**Issue:** The `score_all_directions` WASM function outputs quaternions from `yaw::full_quaternion` in `[w, x, y, z]` order (verified: `quaternion_align` returns `[cos(θ/2), sin(θ/2)·axis]`). However, the web worker (`orient.worker.ts:66`) passes these values directly into `Candidate.quaternion`, and the viewport (`Viewport.ts:253-257`) feeds them to THREE.js `Quaternion.set(x, y, z, w)` which interprets `q[0]` as the X component, not W. The STL export path (`rotate.ts:18`) also expects `[x, y, z, w]`.

Result: candidate orientations displayed in the viewport and exported as STLs use **wrong quaternion components** (W↔X swapped, Y↔Z shifted). For near-identity quaternions this may appear correct, but any non-trivial rotation is displayed/exported incorrectly.

The function's own docstring at line 249 claims the output is `[qx, qy, qz, qw, ...]` (= `[x,y,z,w]`), contradicting the inline comment at line 285 which says `[w, x, y, z]`. The docstring matches the web convention but the code does not.

**Fix:** Either:
(a) Reorder the quaternion output to match the docstring/web convention:
```rust
out.extend_from_slice(&[
    q[1], q[2], q[3], q[0],              // quaternion [x, y, z, w] — three.js convention
    c.overhang, c.footprint, c.max_cross, c.surface_quality, c.height,
    shadowed,
    stable_f, stab.margin, stab.contact_area,
]);
```
Or (b) Have the worker swap components when building the Candidate:
```typescript
quaternion: [metrics[base + 1], metrics[base + 2], metrics[base + 3], metrics[base]],
```

### CR-03: Conflicting docstrings on `score_all_directions` mask the quaternion bug

**File:** `core/src/lib.rs:249 vs 285`
**Issue:** The top-level docstring says `[qx, qy, qz, qw, ...]` (xyzw order) while the inline comment says `// quaternion [w, x, y, z]`. These cannot both be correct. A developer reading only the docstring would implement the consumer assuming `[x,y,z,w]`, which would happen to be right for the web but wrong for the actual code output. The contradiction makes the correct interpretation ambiguous without tracing into `yaw.rs`.

**Fix:** Align both comments with the chosen fix from CR-02. If the code is changed to output `[x,y,z,w]`, update the inline comment:
```rust
q[1], q[2], q[3], q[0],              // quaternion [x, y, z, w] — three.js convention
```

## Warnings

### WR-01: `compute_norm_bounds` docstring says "10 floats", code returns 12

**File:** `core/src/lib.rs:371-373`
**Issue:** The docstring states "Returns 10 floats: [lo[5], hi[5]]" implying 5 metrics. The actual code allocates `[f32::INFINITY; 6]` and returns 12 floats (6 lo + 6 hi) including shadowed. The web consumer (`AppController.ts:217`) correctly uses `subarray(0, 6)` and `subarray(6, 12)`, so the code works, but the docstring will mislead future developers.

**Fix:**
```rust
/// Returns 12 floats: [lo[6], hi[6]] for overhang, footprint, cross, surface, height, shadowed.
```

### WR-02: `score_orientation` docstring says 8 values, code returns 9

**File:** `core/src/lib.rs:183-184`
**Issue:** The docstring says `Returns 8 floats: [dir_x, dir_y, dir_z, overhang, footprint, max_cross, surface, height]` but the code at line 209-212 returns 9 values including `c.shadowed`. The test at line 655 asserts `out.len() == 9` and the consumer at `AppController.ts:239` destructures all 9 values. Stale docstring.

**Fix:**
```rust
/// Returns 9 floats:
///   [dir_x, dir_y, dir_z, overhang, footprint, max_cross, surface, height, shadowed]
```

### WR-03: `--with-identity --all-rankings` truncates output and misreports count

**File:** `core/src/main.rs:237,310,389`
**Issue:** `n_dirs` is computed at line 237 from `od.directions` BEFORE the identity direction is optionally prepended at line 245. In the `all_rankings` path, the candidate output at line 310 iterates `(0..n_dirs)` which excludes the identity candidate, while the `rankings` entries at line 297 reference indices up to `n_dirs` (including identity). This means: (a) the identity candidate's metrics are computed but never included in the JSON candidates array, (b) rankings reference a candidate index that doesn't exist in the output, and (c) `meta.candidate_count` at line 389 is off by 1.

**Fix:** Replace `n_dirs` with `dirs.len()` after the identity insert for all iteration and output:
```rust
let dir_count = dirs.len();  // use this everywhere instead of n_dirs
```

### WR-04: `_exclude_unstable` parameter in `score_all_directions` is accepted but ignored

**File:** `core/src/lib.rs:259`
**Issue:** The WASM function signature accepts `_exclude_unstable: bool` but never uses it. The stability check results are always included in the output regardless of this parameter. Callers (`orient.worker.ts:37`) pass `config.excludeUnstable` expecting unstable candidates to be filtered. The filtering actually happens later in `select_diverse`, so the behavior is accidentally correct, but the parameter is misleading API surface.

**Fix:** Either remove the parameter from the WASM API, or implement the filtering inside the function (skip unstable candidates from the output). If removed, update the .d.ts bindings and worker call site.

### WR-05: Shadowed overhang computed twice per direction in `score_all_directions`

**File:** `core/src/lib.rs:278-279`
**Issue:** `score_components` (line 278) already calls `shadowed_overhang_fraction` internally (scoring.rs:221) with the same parameters (grid_res=32, tol=0.02). Line 279 then calls it again separately and uses that result, discarding `c.shadowed`. This doubles the most expensive per-candidate computation (O(N) height-field build + query) for no benefit.

**Fix:** Remove the redundant call and use `c.shadowed`:
```rust
let c = scoring::score_components(&best_dir, &mesh, critical_angle_deg, 64);
// Use c.shadowed instead of recomputing
let shadowed = c.shadowed;
```

## Info

### IN-01: CLI `--weights` help text says "Five weights" — should be six

**File:** `core/src/main.rs:57`
**Issue:** The comment `/// Five weights: overhang footprint cross surface height` is stale; the system now has 6 metrics including shadowed. This is cosmetic but will confuse CLI users.

**Fix:**
```rust
/// Six weights: overhang footprint cross surface height shadowed
```

### IN-02: Redundant stability recomputation in CLI pipeline

**File:** `core/src/main.rs:271-275`
**Issue:** The `score_one` function at line 256 already computes stability, but only the `stable` bool is kept (line 259). Lines 271-275 then recompute stability for all directions to get `margin` and `contact_area`. This doubles the stability check work. Not a correctness bug, just wasted cycles.

**Fix:** Return the full `StabilityResult` from `score_one` instead of just the bool.

---

_Reviewed: 2026-07-15T12:00:00Z_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_
