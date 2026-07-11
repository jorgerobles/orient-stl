# Requirements: orient-stl

**Defined:** 2026-07-11
**Core Value:** Generate a reliable orientation ranking that minimizes supports and maximizes print success, without the user manually rotating the model.

## v1 Requirements

### STL Loading

- [ ] **STL-01**: User can select a binary STL file via file picker
- [ ] **STL-02**: User can drag-and-drop an STL file onto the page
- [ ] **STL-03**: Binary STL is parsed in Rust via stl-io, raw bytes passed to WASM
- [ ] **STL-04**: Parsed mesh data (vertices, normals) is accessible to both WASM computation and JS viewport

### Orientation Computation

- [ ] **ORIENT-01**: Per-triangle normal, area, centroid precomputed once on load
- [ ] **ORIENT-02**: Convex hull computed from mesh vertices (vendored quickhull)
- [ ] **ORIENT-03**: Candidate directions generated from hull face normals (hull mode)
- [ ] **ORIENT-04**: Candidate directions deduplicated by angular proximity (configurable threshold)
- [ ] **ORIENT-05**: Overhang penalty scored for each candidate (area-weighted, S² space, configurable critical angle 30-35°)
- [ ] **ORIENT-06**: Binary stability check (CoM projected inside contact footprint; rejects unstable orientations)
- [ ] **ORIENT-07**: Results sorted by composite score, returned as `Candidate[]` with quaternion, penalties, stability
- [ ] **ORIENT-08**: Default yaw computed (minimize XY bounding box via rotating calipers)

### Viewport & Navigation

- [ ] **VIEW-01**: three.js viewport displays the model at the current candidate orientation
- [ ] **VIEW-02**: User can navigate to next/previous candidate via buttons or keyboard
- [ ] **VIEW-03**: Orientation change is instant (mesh.quaternion.copy), no geometry reload
- [ ] **VIEW-04**: User can orbit/pan/zoom freely around the model
- [ ] **VIEW-05**: Viewport shows current candidate's rank and score

### Yaw Adjustment

- [ ] **YAW-01**: Circular dial control for yaw rotation around the vertical axis
- [ ] **YAW-02**: Snap candidates derived from rotating calipers (bbox minima + edge aligns)
- [ ] **YAW-03**: Magnetic snap when drag is within configurable threshold of a candidate
- [ ] **YAW-04**: Numeric input for exact yaw angle
- [ ] **YAW-05**: "Reset to auto" button restores default bbox-minimizing yaw

### Export

- [ ] **EXPORT-01**: Export current orientation as a single binary STL (quaternion applied)
- [ ] **EXPORT-02**: Exported STL includes the transformed vertex positions

### Config & Settings

- [ ] **CONFIG-01**: Critical angle configurable (default 30°, range 20-50°)
- [ ] **CONFIG-02**: Mode selector: hull (default) — sphere mode deferred to v2

## v2 Requirements

### Enhanced Scoring

- **ORIENT-09**: Height-weighted overhang penalty (height above lowest point in orientation)
- **ORIENT-10**: `hull_plus_sphere` candidate generation (Fibonacci sphere sampling, N configurable)
- **ORIENT-11**: S² hill-climbing refinement on top-K candidates (configurable iterations)
- **ORIENT-12**: Multi-metric sorting UI (sort by overhang/height/stability independently)

### Viewport Enhancements

- **VIEW-06**: Overhang heatmap visualization (face colors by penalty contribution)
- **VIEW-07**: Side-by-side or overlay comparison of two candidates

## v3 Requirements

### Thumbnails

- **THUMB-01**: Offscreen rendering of each candidate as PNG thumbnail (top-N by score, N configurable)
- **THUMB-02**: Thumbnail strip showing all candidates with score badges
- **THUMB-03**: Click thumbnail to jump to that candidate

### Favorites & Persistence

- **FAV-01**: User can mark candidates as favorites
- **FAV-02**: Favorites persisted to IndexedDB (quaternion + thumbnail blob + metrics)
- **FAV-03**: Favorites restored on page reload for the same model

### Multi-Export

- **EXPORT-03**: Export all favorites as a ZIP bundle (fflate)
- **EXPORT-04**: Individual STLs in ZIP named `model_orientNN_scoreX.stl`

## Out of Scope

| Feature | Reason |
|---------|--------|
| GCode / slicing output | Use existing slicers; output oriented STLs |
| Support generation | Minimize supports via orientation, don't generate them |
| Mesh repair / decimation | Assume clean manifold STLs |
| Cloud / network features | All computation in-browser, no server |
| Real-time orientation drag | Pre-compute ranked list, navigate instantly |
| Multi-model packing | Orient one model at a time |
| ASCII STL support | Deferred unless users need it (binary is standard) |
| Native desktop app | Browser-first with WASM core |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| STL-01 | Phase 1 | Pending |
| STL-02 | Phase 1 | Pending |
| STL-03 | Phase 1 | Pending |
| STL-04 | Phase 2 | Pending |
| ORIENT-01 | Phase 1 | Pending |
| ORIENT-02 | Phase 1 | Pending |
| ORIENT-03 | Phase 1 | Pending |
| ORIENT-04 | Phase 1 | Pending |
| ORIENT-05 | Phase 1 | Pending |
| ORIENT-06 | Phase 1 | Pending |
| ORIENT-07 | Phase 1 | Pending |
| ORIENT-08 | Phase 1 | Pending |
| VIEW-01 | Phase 2 | Pending |
| VIEW-02 | Phase 2 | Pending |
| VIEW-03 | Phase 2 | Pending |
| VIEW-04 | Phase 2 | Pending |
| VIEW-05 | Phase 2 | Pending |
| YAW-01 | Phase 2 | Pending |
| YAW-02 | Phase 2 | Pending |
| YAW-03 | Phase 2 | Pending |
| YAW-04 | Phase 2 | Pending |
| YAW-05 | Phase 2 | Pending |
| EXPORT-01 | Phase 2 | Pending |
| EXPORT-02 | Phase 2 | Pending |
| CONFIG-01 | Phase 1 | Pending |
| CONFIG-02 | Phase 1 | Pending |
| ORIENT-09 | Phase 3 | Pending |
| ORIENT-10 | Phase 3 | Pending |
| ORIENT-11 | Phase 3 | Pending |
| ORIENT-12 | Phase 3 | Pending |
| VIEW-06 | Phase 3 | Pending |
| VIEW-07 | Phase 3 | Pending |
| THUMB-01 | Phase 4 | Pending |
| THUMB-02 | Phase 4 | Pending |
| THUMB-03 | Phase 4 | Pending |
| FAV-01 | Phase 4 | Pending |
| FAV-02 | Phase 4 | Pending |
| FAV-03 | Phase 4 | Pending |
| EXPORT-03 | Phase 4 | Pending |
| EXPORT-04 | Phase 4 | Pending |

**Coverage:**
- v1 requirements: 26 total
- Mapped to phases: 26
- Unmapped: 0 ✓

---
*Requirements defined: 2026-07-11*
*Last updated: 2026-07-11 after initial definition*
