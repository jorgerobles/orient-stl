# Milestone: Support Generation + Unix-Style WASM Architecture

**Date**: 2026-08-27
**Status**: Planned — ready for execution
**Depends on**: Phase 7 completion (or can proceed in parallel)

---

## Milestone Goal

Add resin 3D printing support generation to orient-stl. Not replicate Lychee — build something smarter: fewer supports, volume-aware classification, curvature-aware placement, line-connected raft. Restructure as Unix-style WASM modules for composability.

---

## Architecture

```
crates/
├── stl-parse/          # &[u8] → positions[]
├── stl-repair/         # positions[] → positions[] (repaired)
│   └── depends on geometry-kernel (rlib)
├── mesher/             # positions[] → [positions, normals, areas]
├── orient/             # [positions, normals, areas] → directions[] (ranked)
├── support/            # [positions, normals, areas, direction] → supports[], raft[] (NEW)
│   ├── island.rs       # 2D slice rasterization + connected components
│   ├── volume.rs       # volume_above via BVH ray casting
│   ├── placement.rs    # Poisson-disk + edge seeding
│   ├── raft.rs         # line-connected raft (MST + Delaunay)
│   └── types.rs
└── geometry-kernel/    # rlib only (no more cdylib/WASM bindings)

web/src/
├── workers/
│   ├── stl-parse.worker.ts
│   ├── stl-repair.worker.ts
│   ├── orient.worker.ts
│   ├── support.worker.ts
│   └── export.worker.ts
├── pipeline.ts         # chains workers like Unix pipes
├── viewport/SupportRenderer.ts  # renders support geometry
└── views/SupportPanel.ts        # support toggle + config UI
```

---

## Phases

### Phase 8: Workspace Split (3 plans, waves 1-3)

**Goal**: Restructure coupled monolith into independent WASM modules

| Plan | Wave | Objective | Tasks |
|------|------|-----------|-------|
| 08-01 | 1 | Create workspace + crate scaffolding + move source files | 2 |
| 08-02 | 2 | Create JS workers + pipeline.ts + Makefile | 3 |
| 08-03 | 3 | Delete old core/ + verify + fix imports | 3 |

**Key risk**: Import path changes cascade through TS codebase. Mitigated by systematic grep + fix in Plan 03.

### Phase 9: Support Module (2 plans, waves 1-2)

**Goal**: Implement support generation algorithms in standalone Rust crate

| Plan | Wave | Objective | Tasks |
|------|------|-----------|-------|
| 09-01 | 1 | Core algorithms: types, island, volume, placement, raft | 2 |
| 09-02 | 2 | WASM bindings + support worker + pipeline integration | 2 |

**Key risk**: Island detection accuracy on complex meshes. Mitigated by unit tests on known geometries.

### Phase 10: UI Integration (3 plans, waves 1-3)

**Goal**: UI controls, viewport preview, export with supports

| Plan | Wave | Objective | Tasks |
|------|------|-----------|-------|
| 10-01 | 1 | SupportPanel + AppState + AppController wiring | 1 |
| 10-02 | 2 | SupportRenderer (three.js columns + raft) | 2 |
| 10-03 | 3 | Export with supports merged into STL | 2 |

**Key risk**: Export STL validity with merged geometry. Mitigated by slicer verification checkpoint.

---

## Wave Summary

| Wave | Plans | Parallel | Dependencies |
|------|-------|----------|--------------|
| 1 | 08-01 | — | Phase 7 |
| 2 | 08-02 | — | 08-01 |
| 3 | 08-03 | — | 08-02 |
| 4 | 09-01 | — | 08-03 |
| 5 | 09-02 | — | 09-01 |
| 6 | 10-01 | — | 09-02 |
| 7 | 10-02 | — | 10-01 |
| 8 | 10-03 | — | 10-02 |

**Total**: 8 plans, 8 waves, 18 tasks
**Estimated context**: ~50% per plan (2-3 tasks each)

---

## Success Criteria

### Phase 8 (Workspace Split)
- [ ] 5 independent crates under `crates/`
- [ ] geometry-kernel is rlib-only
- [ ] Each crate builds to its own WASM binary
- [ ] JS workers load WASM independently
- [ ] pipeline.ts chains workers
- [ ] All existing tests pass
- [ ] CLI binary works
- [ ] App behavior identical to pre-split

### Phase 9 (Support Module)
- [ ] `crates/support/` with 5 source files
- [ ] Island detection on known geometries
- [ ] Volume classification: Light < 50mm³, Medium < 500mm³, Heavy > 500mm³
- [ ] Poisson-disk contact placement with edge seeding
- [ ] Line-connected raft (convex hull + Delaunay + MST)
- [ ] WASM binary compiles
- [ ] Unit tests pass with ground truths
- [ ] Pipeline integration works

### Phase 10 (UI Integration)
- [ ] Support toggle + config panel
- [ ] Support columns in viewport (colored by type)
- [ ] Raft mesh in viewport (semi-transparent)
- [ ] Toggle visibility
- [ ] Export with supports baked in
- [ ] Exported STL valid in slicers

---

## Decisions Honored

| ID | Decision | Plan |
|----|----------|------|
| D-01 | 2 WASM binaries (orient + support) — maintains Unix philosophy | 09-02 |
| D-02 | geometry-kernel as rlib shared — repair logic worth sharing | 08-01 |
| D-03 | Restructure FIRST — avoid building on coupled base | 08-01 |
| D-04 | Export returns separate geometry — JS decides how to combine | 10-03 |

---

## File Structure After Milestone

```
orient-stl/
├── Cargo.toml              # workspace root
├── crates/
│   ├── stl-parse/
│   ├── stl-repair/
│   ├── mesher/
│   ├── orient/
│   ├── support/
│   └── geometry-kernel/
├── web/
│   ├── src/
│   │   ├── workers/         # 5 workers
│   │   ├── pipeline.ts      # Unix-pipe orchestrator
│   │   ├── viewport/SupportRenderer.ts
│   │   └── views/SupportPanel.ts
│   └── pkg/                 # 5 WASM binaries
├── Makefile                 # per-crate WASM builds
└── .planning/
    └── phases/
        ├── 08-workspace-split/
        ├── 09-support-module/
        └── 10-ui-integration/
```

---

## Execution

Start with: `/gsd-execute-phase 8`

Phase 8 must complete before Phase 9. Phase 9 must complete before Phase 10. Within each phase, plans execute in wave order.
