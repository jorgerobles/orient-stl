---
title: Support columns not touching mesh — placement + scale-aware rendering
date: 2026-08-27
priority: high
status: open
context: Reported after testing Generate Supports at localhost:5173. Columns render but float away from the mesh; nothing usable for visual validation.
phase: 10-ui-integration
---

**Symptom:** After Generate Supports, support columns do not touch the mesh nor
appear near it. No contact balls / tips visible at the mesh surface. Suspected
coordinate bug plus scale mismatch (column radii are absolute mm, not relative
to model size).

**Expected:**
1. Every column's tip sits ON the mesh surface (the overhang contact point).
2. Column base sits on the raft/plate plane below the contact.
3. Contact markers (small tip) visible at each touch point.
4. Column/radii scale with model size (a 20 mm part and a 200 mm part both look
   right); current `MIN_COLUMN_RADIUS = 0.8` / sphere radius 2 are absolute.
5. Visual quality comparable to Lychee: tapered pillar, tip at contact, wider
   foot at base, colored by type (light/medium/heavy).

**Known root cause (placement) — from `.planning/debug/phase10-support-rendering.md`:**
`crates/support/src/placement.rs` `create_contact_point` mixes coordinate
spaces: the ray origin is built from 2D-grid X/Y plus a *projected height*
(`island.z_max`) as if it were mesh Z:

```rust
let ray_origin = [point_2d[0], point_2d[1], island.z_max + 1.0];
//                        ^^^grid X      ^^^grid Y   ^^^projected height, NOT mesh Z
```

The grid rasterizes in raw mesh XY, but heights are projections along the build
direction — the ray starts from a phantom 3D point, so intersections land
slightly outside the real bbox (evidence: contacts at Z ≈ 26 vs mesh Z range
[-10.6, 22.0]).

**Fix approaches (from the debug doc):**
- **Option A**: un-project — build the ray origin in true mesh space by casting
  from a point far above the mesh along `-dir` (direction is arbitrary, not
  axis-aligned; island detection currently assumes Z-aligned directions too —
  check `island.rs`).
- **Option B (simpler, recommended)**: skip the 2D grid → ray round-trip for
  contact placement. Find mesh triangles whose centroids fall inside the
  island's projected region and use their surface positions directly.

**Lychee reference (`resources/lychee/`):** Lychee Slicer's app image is
checked in at `resources/lychee/squashfs-root/`. Relevant pieces:
- Support *generation* is native/compiled: `resources/app.asar` →
  `node_modules/@mango3d/addon_auto_support/build/Release/autosupport.node`
  (closed binary — API shape only, not portable).
- Support *rendering* is in the JS bundle. Extract with:
  ```bash
  npx @electron/asar extract-file resources/lychee/squashfs-root/resources/app.asar bin/render3D.js
  # writes render3D.js (34 MB, minified) into CWD — keep it OUT of the repo
  ```
  Search for: `_getBaseJoinConeGeometry`, `generateSupportsGrid`,
  `InstancedPool`/`removeInstancedPoolItemBaseFloorJoin` (instanced pillar
  pools), `bedRound` base foot geometry (`CylinderGeometry(l/2+5, l/2+5, 5, 50)`),
  and shape taxonomy in `bin/static/images/support_image_*.svg`
  (base, baseTip, general, mid, mini, tip, bracing) — use as the visual target
  for pillar anatomy.
- Color scheme reference: island `#D55E00`, secondary `#CC79A7`,
  overhang `#F0E442`, stabilization `#009E73`, opacity 0.7.

**Scope of work:**
1. `crates/support/src/placement.rs` — fix contact placement (Option A or B);
   add Rust unit test asserting every contact position lies inside the mesh
   bbox (with small epsilon) for a non-Z-aligned direction.
2. `crates/support/src/island.rs` — verify/fix projection math for arbitrary
   build directions (currently Z-aligned only per debug doc).
3. `web/src/viewport/SupportRenderer.ts` — scale-aware rendering:
   - derive pillar radius from `boundingRadius` (e.g. 0.8–1.5% of it) instead
     of absolute mm; keep per-type ratios;
   - tapered column (tip radius → base radius factor), small tip marker at the
     contact point, wider foot at base;
   - per-type colors (light/medium/heavy) at ~0.7 opacity.
4. Verify end-to-end with the Puppeteer script (`web/test-supports.mjs`) on a
   real STL: screenshot must show columns touching the mesh.

**Constraints:**
- WASM rebuild rule applies (any `crates/support/src/*.rs` change):
  `wasm-pack build crates/support --target bundler --out-dir ../../web/pkg/support`
- Keep `cargo test -p support` green; run `cd web && npx tsc --noEmit && npx vitest run`.

**Acceptance criteria:**
- [ ] All contact tips within mesh bbox (epsilon 0.5 mm) — Rust test
- [ ] Visual: columns visibly touch overhangs on a test STL (Puppeteer screenshot)
- [ ] Radii look correct on both a small (<30 mm) and large (>150 mm) model
- [ ] No regression in candidate count/latency (supports path only — decimated)
