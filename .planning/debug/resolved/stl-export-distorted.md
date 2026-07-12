---
status: resolved
trigger: "stl export distorted"
created: "2026-07-12T00:00:00Z"
updated: "2026-07-12T21:36:00Z"
---

## Current Focus

hypothesis: "CONFIRMED — `rotatePositions()` in web/src/main.ts:378-388 has an incorrect quaternion sandwich-product formula. Replaced with the canonical `t = 2*(q.xyz × v); v' = v + q.w*t + (q.xyz × t)` form, extracted to web/src/rotate.ts for testability."
test: "9 unit tests in web/src/rotate.test.ts — all GREEN. Covers identity round-trip, 180° about X/Y/Z, 90° about Y (right-handed), vector-length preservation (orthogonality), three.js Quaternion.applyToVector parity across 6 quaternions, empty input, and non-mutation."
expecting: "All tests pass. Verified."
next_action: "DONE — fix applied, tests + tsc + build all green."

## Symptoms

expected: "The exported STL should match the rotated/oriented mesh shown in the viewport (same shape, scale, orientation)"
actual: "Exported STL is stretched/squished (non-uniformly scaled/sheared along axes) and does not match the on-screen rotation"
errors: "None — export completes silently; file is just geometrically wrong"
reproduction: "Any STL + default orientation. Click export. Happens on every export regardless of model or chosen candidate."
started: "Never worked — broken since Phase 2 export plan (02-03 Export)"

## Eliminated

- exportSTL.ts (binary STL writer) — pure byte-level passthrough, no math. Confirmed correct; not touched.
- Viewport rotation path — uses three.js Quaternion directly. Correct; not touched.
- centering.ts / compute.ts / orient.worker.ts / loadSTL.ts — unrelated to export rotation. Not touched.
- WASM / Rust core — bug is entirely in TS, no rebuild needed.
- "Convention mismatch" hypothesis — rejected: even identity fails to round-trip, which no convention choice can explain. Confirmed math bug.
- "Non-unit quaternion from candidate source" hypothesis — rejected: identity test fails regardless of input quaternion.

## Evidence

- timestamp: "2026-07-12T00:00:00Z"
  checked: "Export code path — web/src/exportSTL.ts (binary STL writer) and caller in web/src/main.ts:370-376"
  found: "exportSTL.ts itself is a straightforward IEEE-754 little-endian binary STL writer (80-byte header, uint32 triCount, per-triangle: normal + 3 verts + uint16 attr). Geometry just passes through. The caller invokes `exportSTL(rotatePositions(positions, qres), ...)`. The only transformation applied before write is rotatePositions. So distortion must originate in rotatePositions."
  implication: "Bug is isolated to rotatePositions (main.ts:378-388), not to the STL byte writer."

- timestamp: "2026-07-12T00:00:00Z"
  checked: "Mathematical audit of rotatePositions vs canonical quaternion rotation. Convention inferred as q=[x,y,z,w] (three.js standard), identity=[0,0,0,1]."
  found: "The function computes intermediates: qx_=q.y*z-q.z*y, qy_=q.z*x-q.x*z, qz_=q.x*y-q.y*x (these are components of q.xyz × v — correct), BUT qw_=-q.y*x-q.z*y-q.w*z. The dot-product term wrongly uses q[3] (=q.w, the SCALAR) for the z slot instead of q[2] (=q.z, the vector z-component). Should be -q.x*x-q.y*y-q.z*z. Furthermore the output assembly terms (the 2*(...) expressions) do not match any standard form of the sandwich product q*v*q⁻¹; cross-checking identity and 180°-Y cases both fail."
  implication: "The whole expression is a scrambled/non-orthogonal linear map. Feeding the IDENTITY quaternion [0,0,0,1] (xyzw) produces out=(x, y, -z) — a Z mirror — instead of (x,y,z). Feeding 180°-Y [0,1,0,0] produces (-x, y+2x, z) instead of (-x, y, -z). A non-orthogonal transform applied to a mesh produces exactly the observed stretched/squished distortion."

- timestamp: "2026-07-12T00:00:00Z"
  checked: "Why the viewport looks correct but the export doesn't. Cross-reference with web/src/centering.ts and the Phase 2 decision 'Full candidate quaternion = qYaw * qAlign(dir, -Y)'."
  found: "Viewport rotation uses three.js Quaternion (mathematically correct). Export path uses the custom buggy rotatePositions function. The two paths diverge — viewport shows the correct rotation; export writes a distorted mesh. This fully accounts for 'export does not match on-screen', 'no errors', 'never worked', 'every export'."
  implication: "Fix is contained to rotatePositions. No change needed in exportSTL.ts, centering.ts, or the viewport pipeline. Replace the function body with the canonical form (see test plan)."

- timestamp: "2026-07-12T21:35:00Z"
  checked: "RED — reproduced bug via extracted module. Copied rotatePositions verbatim from main.ts:378-388 into a new web/src/rotate.ts and ran web/src/rotate.test.ts (9 tests)."
  found: "7 of 9 tests failed on the buggy formula. Failures matched the hand-derived math EXACTLY: identity test 'expected -3 to be close to 3' (Z mirror); 180°-Y 'expected 4 to be close to 2' (y+2x shear); orthogonality 'expected 3.05 to be close to 5' (non-uniform scaling, |out| != |in|); three.js parity 'expected -7.8 to be close to 7.8' (Z sign flip on identity). The 2 trivial passes (empty input, non-mutation) were expected."
  implication: "Bug definitively reproduced and isolated to the rotatePositions body. The test suite independently confirms the analytical derivation."

- timestamp: "2026-07-12T21:36:00Z"
  checked: "GREEN — replaced rotate.ts body with the canonical cross-product form `t = 2*(q.xyz × v); v' = v + q.w*t + (q.xyz × t)`. Hoisted q-component reads out of the loop for clarity. Re-ran full suite + tsc + build."
  found: "ALL 38 tests pass (29 pre-existing centering/compute + 9 new rotate). `npx tsc --noEmit` returns 0 errors. `npm run build` succeeds (only pre-existing three.js chunk-size warning). The three.js parity test — which directly compares against `THREE.Quaternion.applyToVector3` across 6 quaternions (identity, 180° X/Y/Z, 90° X/Y) on a realistic 5-vertex sample — passes bit-identically (5 decimal places)."
  implication: "Export path now mathematically agrees with the viewport path. The exported STL will match the on-screen rotation. Fix complete and verified."

## Resolution

root_cause: "The export-path `rotatePositions` function (formerly main.ts:378-388) implemented a scrambled quaternion sandwich product. The `qw` intermediate wrongly used `q[3]` (= q.w, the scalar) where it should have used `q[2]` (= q.z, the vector z-component), and the output-axis assembly terms did not match any standard form of q ⊗ v ⊗ q*. The resulting 3×3 linear map was non-orthogonal: even the IDENTITY quaternion failed to round-trip (it produced a Z mirror), and 180°-about-Y produced (-x, y+2x, z) instead of (-x, y, -z). Applied to a mesh, this sheared/squished geometry instead of rotating it — exactly the reported 'export distorted' symptom. The viewport looked correct because it uses three.js Quaternion directly; the two paths diverged only on export."
fix: "Extracted rotatePositions into a new testable module web/src/rotate.ts and replaced its body with the canonical cross-product form: `tx,ty,tz = 2*(q.xyz × v); v'.x = x + qw*tx + (qy*tz - qz*ty); v'.y = y + qw*ty + (qz*tx - qx*tz); v'.z = z + qw*tz + (qx*ty - qy*tx)`. This is the standard optimized quaternion rotation, bit-identical to THREE.Quaternion.applyToVector3. Updated main.ts to import the function and deleted the buggy local copy. Followed Red-Green-Refactor per project CLAUDE.md: wrote failing tests first (7 RED), applied the fix (GREEN), then refactored main.ts to consume the new module (REFACTOR)."
verification: "9 new unit tests in web/src/rotate.test.ts all pass: identity round-trip, 180° about X/Y/Z, 90° about Y (right-handed), vector-length preservation (orthogonality — directly catches the bug class), three.js Quaternion.applyToVector3 parity across 6 quaternions on a realistic mesh sample, empty input, and non-mutation. Full suite: 38/38 pass. `npx tsc --noEmit`: 0 errors. `npm run build`: succeeds. No WASM/Rust rebuild needed — fix is TS-only. No manual browser verification needed per project CLAUDE.md (not a hardware project); the three.js parity test is the authoritative cross-check since three.js is the same library the viewport uses."
files_changed:
  - "web/src/rotate.ts (NEW — extracted module, correct formula)"
  - "web/src/rotate.test.ts (NEW — 9 tests, TDD coverage)"
  - "web/src/main.ts (import rotatePositions from ./rotate; deleted buggy local function — net -11 lines)"
