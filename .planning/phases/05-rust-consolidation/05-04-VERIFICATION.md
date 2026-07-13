# Phase 5 Cross-Verification: CLI ↔ Browser UI Parity

**Goal:** Confirm the consolidated Rust pipeline produces identical results through both the CLI binary and the browser UI for the same STL + critical-angle + profile + ranker combination.

---

## 1. Automated Pre-flight Checks

All checks ran via automation (no human involvement) on 2026-07-13.

| Check | Command | Result |
|-------|---------|--------|
| Production bundle | `npm run build` | ✅ Pass (chunk size warning only) |
| Rust ground-truth tests | `cargo test --lib` | ✅ 78 passed, 0 failed |
| JS rendering-layer tests | `npx vitest run` | ✅ 4 files, 38 passed |
| TypeScript type-check | `npx tsc --noEmit` | ✅ 0 errors |
| WASM export: `score_all_directions` | `grep '^export function score_all_directions' web/pkg/orient_core.d.ts` | ✅ Present |
| WASM export: `rank_candidates` | `grep '^export function rank_candidates' web/pkg/orient_core.d.ts` | ✅ Present |
| WASM export: `compute_norm_bounds` | `grep '^export function compute_norm_bounds' web/pkg/orient_core.d.ts` | ✅ Present |
| WASM export: `select_diverse` | `grep '^export function select_diverse' web/pkg/orient_core.d.ts` | ✅ Present |
| TS compute.test.ts deleted | `test ! -f web/src/compute.test.ts` | ✅ Deleted |
| CLI 12 reference outputs | `ls /tmp/opencode/orient-stl-cli-*.json` | ✅ 12 files, all valid JSON |

**Note on WASM export count:** The plan's acceptance criteria uses `grep -cE 'score_all_directions|rank_candidates|compute_norm_bounds|select_diverse'` which returns 6 (not 4) because the `.d.ts` file comments reference related functions. All 4 exports exist as `^export function` declarations — the extra 2 matches are comment references in JSDoc annotations.

---

## 2. CLI Reference Data

12 CLI invocations across 2 STLs × 3 rankers × 2 profiles. Common args:
`--critical-angle 30 --refine-iters 0 --max-candidates 10 --exclude-unstable true`

**Profile → weight mapping:**
- `resin-biased`: `wOverhang=0.5, wFootprint=1.0, wCross=2.0, wSurface=0.5, wHeight=0.5`
- `equal`: `wOverhang=1.0, wFootprint=1.0, wCross=1.0, wSurface=1.0, wHeight=1.0`

**CLI flag mapping (deviation):** The plan assumed `--ranker` and `--profile` flags, but the actual CLI uses `--method` (for ranker) and `--weights` (for profile weights). The actual CLI flags were used to generate reference outputs.

### Comparison Table

Fill the "Browser" columns by opening http://localhost:5173/ in a browser, loading the STL, selecting the ranker + profile, and reading the rank=0 candidate from the candidate list or `window.candidates[0]` in DevTools.

| # | STL | Ranker | Profile | CLI rank=0 Quaternion | CLI rank=0 Score | Browser rank=0 Quaternion | Browser rank=0 Score | Parity |
|---|-----|--------|---------|----------------------|-----------------|--------------------------|---------------------|--------|
| 1 | test-tetrahedron | consensus | equal | `[0.981956, 0.189108, 0.000000, 0.000000]` | 0.174961 | _(fill me)_ | _(fill me)_ | _(Y/N)_ |
| 2 | test-tetrahedron | consensus | resin-biased | `[0.981956, 0.189108, 0.000000, 0.000000]` | 0.174961 | _(fill me)_ | _(fill me)_ | _(Y/N)_ |
| 3 | test-tetrahedron | topsis | equal | `[0.707107, -0.707107, 0.000000, 0.000000]` | 0.550305 | _(fill me)_ | _(fill me)_ | _(Y/N)_ |
| 4 | test-tetrahedron | topsis | resin-biased | `[0.707107, -0.707107, 0.000000, 0.000000]` | 0.794652 | _(fill me)_ | _(fill me)_ | _(Y/N)_ |
| 5 | test-tetrahedron | weights | equal | `[0.529044, 0.166423, 0.000000, 0.832115]` | 2.000000 | _(fill me)_ | _(fill me)_ | _(Y/N)_ |
| 6 | test-tetrahedron | weights | resin-biased | `[0.707107, -0.707107, 0.000000, 0.000000]` | 1.000000 | _(fill me)_ | _(fill me)_ | _(Y/N)_ |
| 7 | Skulled_Wurm_Bird_WOBase | consensus | equal | `[0.517936, 0.178056, -0.075432, 0.833276]` | 0.447928 | _(fill me)_ | _(fill me)_ | _(Y/N)_ |
| 8 | Skulled_Wurm_Bird_WOBase | consensus | resin-biased | `[0.517936, 0.178056, -0.075432, 0.833276]` | 0.448118 | _(fill me)_ | _(fill me)_ | _(Y/N)_ |
| 9 | Skulled_Wurm_Bird_WOBase | topsis | equal | `[0.078338, 0.015341, 0.859539, -0.504798]` | 0.870320 | _(fill me)_ | _(fill me)_ | _(Y/N)_ |
| 10 | Skulled_Wurm_Bird_WOBase | topsis | resin-biased | `[0.078338, 0.015341, 0.859539, -0.504798]` | 0.941096 | _(fill me)_ | _(fill me)_ | _(Y/N)_ |
| 11 | Skulled_Wurm_Bird_WOBase | weights | equal | `[0.751695, 0.636257, 0.073441, 0.157283]` | 1.021682 | _(fill me)_ | _(fill me)_ | _(Y/N)_ |
| 12 | Skulled_Wurm_Bird_WOBase | weights | resin-biased | `[0.078338, 0.015341, 0.859539, -0.504798]` | 0.803810 | _(fill me)_ | _(fill me)_ | _(Y/N)_ |

**CLI reference JSON files are at:** `/tmp/opencode/orient-stl-cli-*.json` (12 files)

---

## 3. Steps to Capture Browser Column

1. **Start the dev server** (already verified to work):
   ```bash
   cd /home/jorge/Sandbox/orient-stl/web
   npm run dev
   ```
   Wait for `VITE vX ready in Yms` — URL is typically `http://localhost:5173/`.

2. **Open the browser** at `http://localhost:5173/`. Confirm no console errors (DevTools → Console).

3. **Load `test-tetrahedron.stl`** via the file picker or drag-drop. Wait for the candidate list to populate (progress bar should complete).

4. **Set profile:** `resin-biased` (default), **ranker:** `consensus` (default). If these are already selected and candidates are shown, the rank=0 data is visible.

5. **Read the browser's rank=0 candidate:**
   - Option A (preferred): Open DevTools Console and run:
     ```js
     JSON.stringify(window.candidates?.[0] ?? 'no candidates')
     ```
   - Option B: Read from the candidate list DOM (first entry in the ranked list)
   - Note: `window.candidates` is a `Candidate[]` array populated by `main.ts` on line 15. If it's undefined in console, the variable may be module-scoped — check the candidate list DOM display for quaternion/matrix and composite score.

6. **Compare** the browser's rank=0 quaternion and compositeScore to the CLI reference in the table above. Quaternions should match within 1e-4 componentwise, compositeScore within 1e-3.

7. **Fill in** the Browser columns and Parity (Y/N) for each row.

8. (Optional) Repeat for the larger STL `resources/Skulled_Wurm_Bird_WOBase.stl` for a stronger parity check.

---

## 4. Human Sign-off

**Verifier:** ____________________

**Date:** ____________________

**Parity assessment:**
- [ ] All 12 rows match (CLI ↔ Browser parity confirmed across all rankers + profiles)
- [ ] Partial match (____ rows verified, ____ rows match, ____ rows differ)
- [ ] Parity FAILED

**If failed, describe the difference (browser quaternion + CLI quaternion + delta):**

```

```

**Notes / Issues encountered:**

```

```

**Final verdict:**
- [ ] APPROVED — Phase 5 consolidation verified, success criterion #8 confirmed
- [ ] BLOCKED — Parity failure, requires investigation before phase can close

---

*File created: 2026-07-13*
*Human verification pending*
