# Handoff: Support Generation + Unix-Style WASM Architecture

**Date**: 2026-08-26
**Status**: Research complete, awaiting decisions before implementation

---

## 1. What We Want

Add support generation to orient-stl. Not replicate Lychee — build something smarter:
- Fewer supports (Lychee over-generates ~26%)
- Volume-aware: type of support (Light/Medium/Heavy) depends on mass above
- Curvature-aware: circular features get denser support at bottom (suction zone)
- Line-connected raft: most efficient adhesion/resin ratio

Restructure as Unix-style WASM modules: small, independent, composable.

---

## 2. Current State

**Coupled monolith**: One WASM binary (`orient-core`) bundles STL parsing, repair, mesh, hull, candidates, scoring, ranking, selection, stability, yaw.

```
loadSTL.ts calls 6 sequential WASM functions:
  parse → repair → winding → weld → fill → hull+candidates

orient.worker.ts bundles:
  score_all_directions → rank_candidates → select_diverse
```

**Two separate WASM binaries exist** that overlap:
- `orient-core` (core/) — full pipeline
- `geometry-kernel` (geometry-kernel/) — repair + analysis, own WASM bindings

**The data format is already Unix-friendly**: everything is `&[f32]` flat arrays (positions = 9 floats per triangle, normals = 3 per triangle, areas = 1 per triangle). This makes the split natural.

---

## 3. Target Architecture

```
crates/
├── stl-parse/          # &[u8] → positions[]
├── stl-repair/         # positions[] → positions[] (repaired)
│   └── depends on geometry-kernel (rlib)
├── mesher/             # positions[] → [positions, normals, areas]
├── orient/             # [positions, normals, areas] → directions[] (ranked)
│   ├── scoring.rs
│   ├── ranking.rs
│   ├── selection.rs
│   ├── hull.rs
│   ├── candidates.rs
│   ├── stability.rs
│   └── yaw.rs
├── support/            # [positions, normals, areas, direction] → supports[], raft[] (NUEVO)
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
└── pkg/
    ├── stl-parse_bg.wasm
    ├── stl-repair_bg.wasm
    ├── orient_bg.wasm
    └── support_bg.wasm
```

**Every module**: one WASM binary, one job, `&[f32]` in, `&[f32]` out.

---

## 4. Support Module Detail

### Algorithm

1. **Island detection** (2D per-layer rasterization):
   - Rasterize mesh at each layer height → binary grid
   - Mark pixels cured in current layer but not connected to layer above
   - Connected components → Islands

2. **Volume classification** (per island):
   - Cast ray upward, sum `area × distance` of intersected triangles
   - `< 50 mm³` → Light (0.25mm tip, 0.2mm penetration)
   - `50-500 mm³` → Medium (0.40mm tip, 0.3mm penetration)
   - `> 500 mm³` → Heavy (0.80mm tip, 0.4mm penetration)

3. **Contact point placement**:
   - Variable-density Poisson-disk on island surface
   - Edge/corner seeding (extra points at sharp features)
   - Spacing: Light 2.5-6mm, Medium 2-5mm, Heavy 1.5-3.5mm

4. **Circular deformity**:
   - Local curvature × dot(normal, -direction)
   - Bottom of cylinder (cos > 0.7) → Heavy density
   - Sides (0.3 < cos < 0.7) → Medium
   - Top (cos < 0.3) → Light or skip

5. **Line-connected raft**:
   - Convex hull of support bases (2D)
   - Delaunay triangulation → raft mesh
   - MST + extra edges → line connections
   - 0.5-1.5mm thick, 1-2mm line width

### WASM API

```rust
#[wasm_bindgen]
pub fn generate_supports(
    positions: &[f32],
    normals: &[f32],
    areas: &[f32],
    direction: &[f32],  // [dx, dy, dz]
    config: JsValue,     // SupportConfig JSON
) -> JsValue;            // SupportResult JSON
```

---

## 5. Decisions Needed

| # | Question | Options | Default Recommendation |
|---|----------|---------|----------------------|
| 1 | `orient` + `support` en 1 o 2 WASM binaries? | A) 2 binaries (más limpio, 1 round-trip JS extra) / B) 1 binary (más grande, sin round-trip extra) | **A) 2 binaries** — maintains Unix philosophy, round-trip cost is negligible vs WASM compute |
| 2 | ¿Mantener geometry-kernel como rlib compartido? | A) Sí, rlib shared / B) Absorber en stl-repair | **A) rlib shared** — repair logic is complex, worth sharing |
| 3 | ¿Reestructurar workspace PRIMERO o implementar support en core/ primero? | A) Reestructurar primero / B) Support primero en core/, refactor después | **A) Reestructurar primero** — avoid building on a coupled base |
| 4 | ¿Export STL con soportes baked in? | A) Sí, nuevo módulo export-stl / B) Devolver geometría separada, JS ensambla | **B) Separada** — keeps modules independent, JS decides how to combine |

---

## 6. Phases

### Phase 1: Workspace Split (no new features)
- Create `crates/` structure
- Move modules to independent crates
- geometry-kernel → rlib only
- Create per-module WASM builds
- Create JS workers + `pipeline.ts`
- Verify: all tests pass, same behavior

### Phase 2: Support Module
- `crates/support/` with island, volume, placement, raft, types
- WASM bindings
- Unit tests on known geometries (flat overhang, cylinder, sphere)
- Integration with pipeline.ts

### Phase 3: UI Integration
- Support toggle + config panel
- Support preview in viewport
- Export with supports

---

## 7. Key Files to Read

| File | Why |
|------|-----|
| `core/src/lib.rs:66-138` | Current `prepare_data_native` — shows the coupled pipeline |
| `core/src/scoring.rs:238-385` | `shadowed_overhang_fraction` — already builds a 2D height field, similar to island detection |
| `geometry-kernel/src/flat.rs:1-50` | Repair functions that will move to `stl-repair` |
| `web/src/loadSTL.ts:38-146` | Current orchestration — will become `pipeline.ts` |
| `web/src/orient.worker.ts:18-81` | Current worker — will split into per-module workers |
| `.planning/research/SUPPORT-GENERATION.md` | Full research doc |

---

## 8. Risks

1. **wasm-pack per-crate build time**: 5 crates × ~30s = ~2.5min total. Mitigate: parallel builds (`make -j5 wasm`).
2. **WASM binary size**: 5 small binaries vs 1 big one. Each ~50-200KB. Total may be slightly larger due to duplicated serde/etc. Mitigate: `opt-level = "s"`, LTO.
3. **JS orchestration complexity**: pipeline.ts needs to manage 5 workers. Mitigate: simple sequential chain, each worker is a clean function call.
4. **Data copying between workers**: Each worker receives/copies `&[f32]` arrays. For 500K triangles (~18MB positions), this is ~18MB × 5 copies. Mitigate: SharedArrayBuffer (future optimization), or accept the cost since WASM compute dominates.
