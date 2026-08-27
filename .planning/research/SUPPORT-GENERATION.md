# Research: Unix-Style WASM Architecture + Support Generation

**Date**: 2026-08-26
**Goal**: Restructure orient-stl as composable WASM workers (Unix pipes model), add support generation as independent module.

---

## 1. Current Architecture (Coupled)

```
Single WASM binary: orient-core
├── stl.rs          ─ parse STL bytes
├── repair.rs       ─ (via geometry-kernel) repair, winding, weld, fill
├── mesh.rs         ─ precompute normals/areas/vertices
├── hull.rs         ─ convex hull
├── candidates.rs   ─ generate directions
├── scoring.rs      ─ overhang, footprint, cross-section, etc.
├── ranking.rs      ─ rank by weights/consensus/topsis
├── selection.rs    ─ diverse subset
├── stability.rs    ─ footprint/CoM check
├── yaw.rs          ─ quaternion from direction
└── harness.rs      ─ test harness
```

**Problems**:
1. One binary = one compilation unit. Can't swap scoring without rebuilding everything.
2. `loadSTL.ts` orchestrates 6 sequential WASM calls (parse → repair → winding → weld → fill → hull) — tightly coupled to the internal pipeline.
3. `orient.worker.ts` bundles score + rank + select — can't use scoring without ranking.
4. Adding support generation means bloating the same binary further.
5. No reuse: geometry-kernel has its own WASM bindings (separate from orient-core), duplicating code.

---

## 2. Target Architecture: Unix-Style WASM Modules

**Principle**: Each module is a small, self-contained WASM binary with a single responsibility. Modules communicate via flat `&[f32]` arrays — the same data format flows through pipes, like Unix stdio.

```
                    ┌─────────────┐
  file.stl ───────▶│  stl-parse  │──── positions[]
                    └─────────────┘
                          │
                          ▼
                    ┌─────────────┐
  positions[] ────▶│  stl-repair │──── positions[] (repaired)
                    └─────────────┘
                          │
                          ▼
                    ┌─────────────┐
  positions[] ────▶│  mesher     │──── positions[], normals[], areas[]
                    └─────────────┘
                          │
                          ▼
                    ┌─────────────┐
  positions[] ────▶│  orient     │──── directions[] (ranked candidates)
  normals[]  ────▶│  (scoring   │
  areas[]    ────▶│   + ranking)│
                    └─────────────┘
                          │
                          ▼
                    ┌─────────────┐
  positions[] ────▶│  support    │──── supports[], raft[]
  normals[]  ────▶│  (islands + │
  areas[]    ────▶│   placement)│
  direction  ────▶│             │
                    └─────────────┘
                          │
                          ▼
                    ┌─────────────┐
  supports[] ─────▶│  export-stl │──── final STL with supports baked in
  positions[] ────▶│             │
                    └─────────────┘
```

### Module Contracts (input → output)

| Module | Input | Output | Binary |
|--------|-------|--------|--------|
| `stl-parse` | `&[u8]` (file bytes) | `positions[]` (flat f32) | `stl-parse.wasm` |
| `stl-repair` | `positions[]` | `positions[]` (repaired) | `stl-repair.wasm` |
| `mesher` | `positions[]` | `positions[], normals[], areas[]` | `mesher.wasm` |
| `orient` | `positions[], normals[], areas[]` | `directions[], metrics[]` | `orient.wasm` |
| `support` | `positions[], normals[], areas[], direction` | `supports[], raft[]` | `support.wasm` |
| `export-stl` | `positions[], supports[], raft[]` | `&[u8]` (STL bytes) | `export-stl.wasm` |

### Key Design Decision: Flat Arrays, Not Objects

All modules exchange `Vec<f32>` — no serialized structs, no JSON overhead. The JS orchestrator knows the layout (e.g., "every 3 floats = one vertex"), but the WASM modules are agnostic. This is the Unix pipe model: bytes in, bytes out.

```rust
// Every module follows this pattern:
#[wasm_bindgen]
pub fn process(input: &[f32]) -> Vec<f32> {
    // ... transform ...
    output
}
```

---

## 3. Workspace Restructure

```
orient-stl/
├── crates/
│   ├── stl-parse/          # Binary: stl-parse.wasm
│   │   ├── Cargo.toml
│   │   └── src/lib.rs      # parse_stl(&[u8]) -> Vec<f32>
│   │
│   ├── stl-repair/         # Binary: stl-repair.wasm
│   │   ├── Cargo.toml      # depends on: geometry-kernel (rlib)
│   │   └── src/lib.rs      # repair(positions) -> positions
│   │
│   ├── mesher/             # Binary: mesher.wasm
│   │   ├── Cargo.toml
│   │   └── src/lib.rs      # precompute(positions) -> [positions, normals, areas]
│   │
│   ├── orient/             # Binary: orient.wasm (current core, slimmed down)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs      # score_all + rank + select
│   │       ├── scoring.rs
│   │       ├── ranking.rs
│   │       ├── selection.rs
│   │       ├── hull.rs
│   │       ├── candidates.rs
│   │       ├── stability.rs
│   │       └── yaw.rs
│   │
│   ├── support/            # Binary: support.wasm (NEW)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs      # generate_supports()
│   │       ├── island.rs   # 2D island detection
│   │       ├── volume.rs   # volume_above heuristic
│   │       ├── placement.rs # Poisson-disk contact point placement
│   │       ├── raft.rs     # line-connected raft generation
│   │       └── types.rs    # SupportConfig, ContactPoint, Support
│   │
│   └── geometry-kernel/    # Library (rlib), NOT cdylib
│       ├── Cargo.toml      # crate-type = ["rlib"] only
│       └── src/            # shared mesh ops (repair, analysis)
│
├── web/
│   ├── src/
│   │   ├── workers/
│   │   │   ├── stl-parse.worker.ts
│   │   │   ├── stl-repair.worker.ts
│   │   │   ├── orient.worker.ts
│   │   │   ├── support.worker.ts
│   │   │   └── export.worker.ts
│   │   ├── pipeline.ts     # orchestrator: chains workers like Unix pipes
│   │   └── ...
│   └── pkg/                # per-module .wasm files
│       ├── stl-parse_bg.wasm
│       ├── stl-repair_bg.wasm
│       ├── mesher_bg.wasm
│       ├── orient_bg.wasm
│       └── support_bg.wasm
│
├── Makefile                # build each crate independently
└── Cargo.toml              # workspace member paths
```

### Why Not One Big Binary With Feature Flags?

Feature flags (`#[cfg(feature = "support")]`) still produce ONE binary. You can't load "just support" without also loading scoring, ranking, hull, etc. Independent binaries = independent loading, independent caching, independent updates.

---

## 4. Support Module Design (`crates/support/`)

### 4.1 API

```rust
// crates/support/src/lib.rs

#[wasm_bindgen]
pub fn generate_supports(
    positions: &[f32],      // flat triangle vertices
    normals: &[f32],        // per-triangle normals
    areas: &[f32],          // per-triangle areas
    direction: &[f32],      // [dx, dy, dz] build direction
    config: JsValue,        // SupportConfig JSON
) -> JsValue;               // SupportResult JSON
```

### 4.2 Internal Modules

```
support/
├── island.rs       ─ 2D slice-based island detection
│   └── detect_islands(mesh, direction, layer_height) -> Vec<Island>
│
├── volume.rs       ─ volume_above heuristic for support classification
│   └── compute_volume_above(mesh, point, direction) -> f32
│
├── placement.rs    ─ contact point placement (Poisson-disk + edge seeding)
│   └── place_contacts(island, config) -> Vec<ContactPoint>
│
├── raft.rs         ─ line-connected raft generation
│   └── generate_raft(bases, config) -> RaftGeometry
│
└── types.rs        ─ shared data structures
    ├── SupportConfig
    ├── SupportType (Light/Medium/Heavy)
    ├── ContactPoint
    ├── Support
    └── SupportResult
```

### 4.3 Island Detection Algorithm

```
Input: mesh (positions, normals, areas), direction, layer_height
Output: Vec<Island> with centroids, areas, height ranges

for each layer z:
    1. Rasterize mesh at height z → binary grid (cell_size = 0.5mm)
    2. Rasterize mesh at height z + layer_height → grid_above
    3. For each cured pixel in grid:
        - Check if connected to cured pixel in grid_above
        - If NOT → mark as island pixel
    4. Connected components on island pixels → Islands
```

### 4.4 Volume-Aware Classification

For each island, cast ray upward and sum `area × distance` of intersected triangles:
- `< 50 mm³` → Light (0.25mm tip)
- `50-500 mm³` → Medium (0.40mm tip)
- `> 500 mm³` → Heavy (0.80mm tip)

### 4.5 Line-Connected Raft

1. Convex hull of support base points (2D)
2. Delaunay triangulation → raft mesh
3. MST + extra edges → line connections
4. Output: vertices + triangles + lines, 0.5-1.5mm thick

---

## 5. Pipeline Orchestrator (JS)

```typescript
// web/src/pipeline.ts — Unix-pipe-style orchestration

import stlParse from './workers/stl-parse.worker?worker';
import stlRepair from './workers/stl-repair.worker?worker';
import mesher from './workers/mesher.worker?worker';
import orient from './workers/orient.worker?worker';
import support from './workers/support.worker?worker';

interface PipelineResult {
  positions: Float32Array;
  normals: Float32Array;
  areas: Float32Array;
  candidates: Candidate[];
  supports?: SupportResult;
}

export async function runPipeline(
  file: File,
  config: PipelineConfig,
  onProgress: (stage: string, pct: number) => void,
): Promise<PipelineResult> {
  // Stage 1: Parse STL
  const bytes = new Uint8Array(await file.arrayBuffer());
  const positions = await runWorker(stlParse, bytes);
  onProgress('parse', 10);

  // Stage 2: Repair (optional)
  let repaired = positions;
  if (config.autoRepair) {
    repaired = await runWorker(stlRepair, positions);
    onProgress('repair', 30);
  }

  // Stage 3: Mesh (normals, areas)
  const mesh = await runWorker(mesher, repaired);
  onProgress('mesh', 40);

  // Stage 4: Orient (score + rank)
  const orientResult = await runWorker(orient, {
    positions: mesh.positions,
    normals: mesh.normals,
    areas: mesh.areas,
    config: config.orient,
  });
  onProgress('orient', 70);

  // Stage 5: Support (optional)
  let supports = undefined;
  if (config.generateSupports) {
    const bestDir = orientResult.candidates[0].direction;
    supports = await runWorker(support, {
      positions: mesh.positions,
      normals: mesh.normals,
      areas: mesh.areas,
      direction: bestDir,
      config: config.support,
    });
    onProgress('support', 90);
  }

  return { ...mesh, ...orientResult, supports };
}
```

---

## 6. Implementation Plan

### Phase 1: Workspace Split (no new features)
- [ ] Create `crates/` directory structure
- [ ] Move `core/src/stl.rs` → `crates/stl-parse/src/lib.rs`
- [ ] Move `core/src/mesh.rs` → `crates/mesher/src/lib.rs`
- [ ] Move repair logic → `crates/stl-repair/src/lib.rs` (depends on geometry-kernel)
- [ ] Slim down `core/` → `crates/orient/` (scoring, ranking, selection, hull, candidates, stability, yaw)
- [ ] Convert `geometry-kernel` to rlib-only (no more cdylib)
- [ ] Create workspace `Cargo.toml`
- [ ] Update Makefile for per-crate builds
- [ ] Create JS workers for each module
- [ ] Create `pipeline.ts` orchestrator
- [ ] Verify: all existing tests pass, same behavior

### Phase 2: Support Module (new feature)
- [ ] Create `crates/support/` crate
- [ ] Implement `island.rs` (2D slice rasterization + connected components)
- [ ] Implement `volume.rs` (volume_above via BVH ray casting)
- [ ] Implement `placement.rs` (Poisson-disk + edge seeding)
- [ ] Implement `raft.rs` (line-connected raft)
- [ ] WASM bindings
- [ ] Unit tests on known geometries
- [ ] Integration with pipeline.ts

### Phase 3: UI Integration
- [ ] Support toggle in UI
- [ ] Support preview in viewport (render support geometry)
- [ ] Export with supports baked in
- [ ] Config panel for support parameters

---

## 7. Build System

```makefile
# Per-module builds (parallelizable)
wasm-stl-parse:
    wasm-pack build crates/stl-parse --target bundler --out-dir ../../web/pkg/stl-parse

wasm-stl-repair:
    wasm-pack build crates/stl-repair --target bundler --out-dir ../../web/pkg/stl-repair

wasm-mesher:
    wasm-pack build crates/mesher --target bundler --out-dir ../../web/pkg/mesher

wasm-orient:
    wasm-pack build crates/orient --target bundler --out-dir ../../web/pkg/orient

wasm-support:
    wasm-pack build crates/support --target bundler --out-dir ../../web/pkg/support

# Build all
wasm: wasm-stl-parse wasm-stl-repair wasm-mesher wasm-orient wasm-support

# Build specific (for dev)
wasm-support-only:
    wasm-pack build crates/support --target bundler --out-dir ../../web/pkg/support
```

---

## 8. References

- Lychee Island Detection: https://docs.mango3d.io/doc/resin-documentation/resin-support/island/
- Lychee Raft: https://docs.mango3d.io/doc/resin-documentation/resin-prepare/raft/
- PrusaSlicer SLA: `src/libslic3r/SLA/` (AGPL, study only)
- RapidMade SLA guide: https://rapidmade.com/support-generation-for-sla-dlp-lcd-3d-printing
