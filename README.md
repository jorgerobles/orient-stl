# Orient STL

**Browser-based auto-orientation for resin 3D printing.**

Orient STL finds optimal print orientations for STL models by scoring candidate
orientations against heuristics that model real resin-printing failure modes:
overhang penalty, peel-force proxy (max cross-section), footprint area, and
shadowed-overhang fraction. Load an STL, browse the ranked candidates in a
3D viewport, tweak yaw, and export the oriented STL — all client-side, no
upload needed.

## The Problem

In resin (SLA/DLP/LCD) printing, supports are determined by orientation:

- **Overhangs** that point downward at >30–35° from vertical need supports,
  and every support leaves a surface blemish.
- **Peel force** (the suction from FEP film release) scales with the largest
  cross-sectional area parallel to the build plate. Minimising the max peel
  layer reduces delamination risk.
- **Shadowing** — overhangs that sit above other geometry get trapped resin
  and require harder-to-remove supports.
- **Stability** — the model's centre of mass must fall inside its contact
  footprint or it tips during printing.
- **Bed utilisation** — compact XY footprint lets you print more in parallel.

Existing slicers auto-orient but present a single "best" answer with no way to
explore tradeoffs. Orient STL instead generates a diversity-preserving ranked
list of candidates, lets you browse them, re-rank by different criteria
(overhang / footprint / cross-section / shadowing), and pick the right
compromise.

## How It Works

### Architecture

```
STL file → [Rust WASM: parse + decimate + convex hull + candidate directions]
                                              ↓
                          [JS Workers: score each direction in parallel]
                                              ↓
                          [Composite ranking → 3D viewport with three.js]
                                              ↓
                          [Export oriented STL or adjust yaw]
```

Two-tier computation:

1. **Rust WASM (`orient-core` crate)** — one-time geometry prep: STL parsing
   via `stl-io`, mesh precomputation (normals, areas, per-triangle vertices),
   vertex sampling for convex hull (quickhull, vendored ~300 LoC), and
   candidate direction generation (hull face normals or hull + fibonacci sphere
   for complex models).
2. **JS Web Workers** — per-candidate scoring runs in parallel across
   `navigator.hardwareConcurrency - 1` workers. Each worker evaluates all 4
   heuristics, then results are merged with angular diversity filtering and
   composite ranking.

### Scoring Heuristics

All heuristics operate in **S² space** (2 DOF) — the insight is that overhang
score depends only on the `down_local` direction, not full rotation. Yaw (3rd
DOF) is resolved separately after scoring via minimum-bounding-box on the
convex hull.

| Heuristic | Code | What It Models | O |
|-----------|------|----------------|---|
| **H1 — Overhang penalty** | `score_candidate` | Area-weighted penalty for faces exceeding critical angle (default 30°). Uses `cos_i × area × (cos_i - cos_crit)` — bigger faces pointing more downward incur more penalty. | O(N) per direction |
| **H2 — Max cross-section** | `max_cross_section` | Z-histogram approximation: slices the mesh along the candidate axis into bins, finds the bin with the most projected area. Proxy for peak peel force during FEP release. | O(N) per direction |
| **H4 — Footprint area** | `footprint_area` | Sum of absolute projected areas of all faces onto the plane normal to the candidate axis. Larger footprint = better bed adhesion. | O(N) per direction |
| **H11 — Shadowed overhang** | `shadowed_overhang_fraction` | Rasterises a 2.5D height field in the plane perpendicular to the direction, then checks whether each overhang triangle's centroid has a clear path to the build plate or sits above another surface (shadowed → trapped resin risk). | O(N) per direction |

### Composite Score

Default ranking uses **consensus (minimax)**:
- Each metric is min-max normalised across the candidate pool
- Height-weighted overhang penalty (taller models get a k=0.5 multiplier)
- `compositeScore = 1 - max(overhangNorm, footprintNorm, crossNorm, shadowNorm)`
- Ranges 0–1 where 1.0 = best across all metrics

Alternative **weighted-sum re-ranking** is available at the console API:
`rankByWeights(candidates, { wOverhang, wFootprint, wCross })` with presets
for overhang-only, footprint-only, peel-biased, and equal weighting.

### Candidate Generation

Two modes configurable in the UI:

- **`hull`** (default): directions are the face normals of the model's convex
  hull. Efficient for simple models where hull faces approximate good
  orientations.
- **`hull_plus_sphere`**: hull normals + 200 fibonacci-sphere samples,
  deduplicated by angular proximity. Better for organic/articulated models
  (miniatures, figurines) where hull misses subtle overhang directions.

Directions are **deduplicated** at a configurable angle threshold (default 3°).

### Stability Check

Each candidate is checked for physical stability:
1. Project all vertices onto the candidate axis, find the lowest (contact) set
2. Compute 2D convex hull of contact points on the build plate → contact polygon
3. Check if centroid (uniform-density CoM proxy) projects inside the polygon
4. Margin = normalised edge distance from CoM to nearest polygon edge

Unstable candidates are excluded by default (togglable).

### Yaw Control

After selecting an orientation (direction), yaw around the vertical axis is
adjustable via:
- **Gizmo rings** — drag X/Y/Z torus rings in the 3D viewport
- **Camera ring** — outermost ring rotates around camera view axis
- Slider in the UI adjusts yaw incrementally

Default yaw minimises the XY bounding box of the oriented model (rotating
calipers on the hull projection → best platform utilisation).

### Export

Export the current orientation as a binary STL with the bake rotation applied.
Single STL download via Blob/URL.createObjectURL.

## Project Structure

```
orient-stl/
├── core/                    # Rust crate → WASM
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs           # WASM public API (prepare_data, refine_orientation)
│       ├── stl.rs           # Binary STL parser (via stl-io)
│       ├── mesh.rs          # Normal/area/vertex precomputation
│       ├── hull.rs          # Vendored quickhull incremental
│       ├── candidates.rs    # Direction generation + fibonacci sphere + yaw
│       ├── decimate.rs      # Vertex sampling for hull computation
│       ├── scoring.rs       # H1/H2/H4/H11 heuristics
│       ├── stability.rs     # CoM/footprint stability check
│       └── harness.rs       # Internal benchmarks
├── web/                     # Frontend (Vite + three.js + TypeScript)
│   ├── index.html
│   ├── vite.config.ts
│   ├── tsconfig.json
│   ├── package.json
│   └── src/
│       ├── main.ts          # App entry: file handling, worker orchestration
│       ├── viewport.ts      # three.js viewport + gizmo drag interaction
│       ├── compute.ts       # All heuristics (TS), decimation, ranking
│       ├── loadSTL.ts       # WASM init + load bridge
│       ├── exportSTL.ts     # Binary STL writer + download
│       ├── orient.worker.ts # Web Worker entry
│       ├── centering.ts     # Centroid computation
│       └── types.ts         # OrientConfig interface
├── tools/                   # Utilities
│   ├── gen-test-stl.mjs     # Generate test STL files
│   └── test-wasm.mjs        # Node.js WASM integration test
├── resources/               # Test models (STL)
└── orient-spec.md           # Technical specification
```

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) with `wasm32-unknown-unknown` target:
  ```bash
  rustup target add wasm32-unknown-unknown
  ```
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)
- Node.js ≥ 18

### Build & Run

```bash
# Build the WASM core
cd core && wasm-pack build --target bundler --out-dir ../web/pkg && cd ..

# Install web dependencies
cd web && npm install

# Build WASM + frontend
npm run rebuild

# Dev server with hot reload
npm run dev

# Tests
npm test
```

### Build from Scratch (clean)

```bash
cd web && npm run wasm:build && npm run build
```

### Visit

Open `http://localhost:5173` (Vite default). Drag an STL file onto the
drop zone or click to select.

## Design Rationale

### Why Rust → WASM instead of pure JS?

The scoring loop is O(N_triangles × N_candidates). For a 500K-triangle model
with 200 candidates, that's 100M evaluations. WASM runs 2–5× faster than
equivalent JS for tight loops with f32 arithmetic. The split: WASM does the
one-time prep (STL parsing, hull computation, mesh precomputation), while JS
workers handle per-candidate scoring. This keeps WASM boundary crossings to
a minimum.

### Why S² instead of SO(3)?

The overhang score depends only on which direction "down" points, not on the
rotation around that axis (yaw). This turns a 3-DOF search (SO(3)) into a
2-DOF search on the unit sphere (S²). Candidate generation, scoring, and
refinement all work in S²; yaw is resolved independently after selection
via a minimum-bounding-box criterion on the convex hull projection.

### Why vendored quickhull?

The convex hull is computed once per model to generate candidate directions.
Using a vendored ~300-line quickhull avoids pulling in `ndarray`, `rayon`, or
native `qhull` bindings that complicate wasm32-unknown-unknown compilation and
bloat binary size. The hull only needs face normals, not a full mesh topology.

### Why decimate for scoring?

Precomputed per-triangle data (normals, areas, centroids) for a 500K-triangle
mesh takes ~12MB of f32 arrays. Decimating to ~12K triangles preserves
orientation rank order while reducing per-candidate evaluation from 500K to
12K intersections — a 40× speedup. The decimation is uniform-stride (every
Nth triangle), not error-metric (no quadric error needed for rank preservation).

### Why no server?

All computation happens client-side: Rust → WASM for geometry prep, JS workers
for scoring, three.js for rendering, Blob API for export. No STL data ever
leaves the browser. The only external dependency is the initial WASM binary
load.

### Why consensus (minimax) ranking over weighted sum?

A weighted sum buries tradeoffs: a candidate with great overhang but terrible
peel force can score high if the overhang weight dominates. Consensus ranking
takes each candidate's worst normalised metric as its score — the winner is
the orientation where *all* metrics are good rather than one being excellent
and another terrible. The weighted-sum API is available for advanced users
who want to tune the tradeoff.

## Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| Overhang score = cos_i - cos_crit, not binary threshold | Continuous penalty gradient enables ranking, not just pass/fail |
| Centroid (vertex avg) as CoM proxy | Uniform density assumption; good enough for stability — exact CoM requires volume integration |
| Yaw resolved after scoring | Decouples 2-DOF orientation search from 1-DOF platform utilisation |
| Workers split by direction, not mesh | No cross-worker state; each worker scores a chunk of candidates independently |
| stl-io crate for parsing | Zero dependencies, works with `Cursor<&[u8]>` in WASM, handles binary STL spec correctly |
| `refine_orientation` in WASM | Hill-climbing in S² perturbs direction + re-scored in WASM; only the WASM crate has the full mesh data without JS marshal overhead |

## Browser Support

Targets modern browsers with WebAssembly support (Chrome, Firefox, Safari,
Edge — all post-2020). No Internet Explorer.

## License

MIT
