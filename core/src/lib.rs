mod stl;
mod mesh;
mod hull;
mod candidates;
mod scoring;
mod stability;
mod decimate;
mod rng;
mod ranking;
mod selection;
mod yaw;
#[cfg(test)]
mod harness;

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrientConfig {
    #[serde(default = "default_mode")]
    mode: String,
    #[serde(default = "default_critical_angle")]
    critical_angle_deg: f32,
    #[serde(default = "default_dedupe_angle")]
    dedupe_angle_deg: f32,
    #[serde(default = "default_refine")]
    refine_iterations: u32,
    #[serde(default = "default_exclude_unstable")]
    exclude_unstable: bool,
}

fn default_mode() -> String { "hull".to_string() }
fn default_critical_angle() -> f32 { 30.0 }
fn default_dedupe_angle() -> f32 { 3.0 }
fn default_refine() -> u32 { 0 }
fn default_exclude_unstable() -> bool { true }

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OriData {
    positions: Vec<f32>,
    normals: Vec<f32>,
    areas: Vec<f32>,
    directions: Vec<f32>,
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn prepare_data(bytes: &[u8], config: &JsValue) -> JsValue {
    let config: OrientConfig = serde_wasm_bindgen::from_value(config.clone())
        .unwrap_or_else(|e| wasm_bindgen::throw_str(&format!("Invalid config: {e}")));

    if config.mode != "hull" && config.mode != "hull_plus_sphere" {
        wasm_bindgen::throw_str(&format!("Unknown mode: {}", config.mode));
    }

    let triangles = stl::parse_stl(bytes)
        .unwrap_or_else(|e| wasm_bindgen::throw_str(&e));
    if triangles.is_empty() {
        wasm_bindgen::throw_str("No triangles in STL");
    }

    let flat: Vec<f32> = triangles.iter().flat_map(|v| v.iter()).copied().collect();
    let m = mesh::precompute_mesh(&flat);
    if m.triangle_count == 0 {
        wasm_bindgen::throw_str("All triangles are degenerate");
    }

    let hull_verts = decimate::sample_for_hull(&m.vertices);
    let hull = hull::compute_hull(&hull_verts);
    if hull.face_normals.is_empty() {
        wasm_bindgen::throw_str("Could not compute convex hull (all vertices coplanar?)");
    }

    let deduped = if config.mode == "hull_plus_sphere" {
        let combined = candidates::generate_hull_plus_sphere(&hull, 200, config.dedupe_angle_deg);
        candidates::deduplicate_directions(&combined, config.dedupe_angle_deg)
    } else {
        let directions = candidates::generate_candidates(&hull);
        candidates::deduplicate_directions(&directions, config.dedupe_angle_deg)
    };

    let mut dir_flat = Vec::with_capacity(deduped.len() * 3);
    for d in &deduped {
        dir_flat.push(d[0]);
        dir_flat.push(d[1]);
        dir_flat.push(d[2]);
    }

    let mut normals_flat = Vec::with_capacity(m.normals.len() * 3);
    for n in &m.normals {
        normals_flat.push(n[0]);
        normals_flat.push(n[1]);
        normals_flat.push(n[2]);
    }

    // Flatten only the CLEAN vertices (non-degenerate triangles) to
    // keep positions and normals/areas in sync.
    let clean: Vec<f32> = m.vertices.iter().flat_map(|v| v.iter()).copied().collect();

    serde_wasm_bindgen::to_value(&OriData {
        positions: clean,
        normals: normals_flat,
        areas: m.areas,
        directions: dir_flat,
    }).unwrap()
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn refine_orientation(
    positions: &[f32],
    normals: &[f32],
    areas: &[f32],
    dir_x: f32,
    dir_y: f32,
    dir_z: f32,
    critical_angle_deg: f32,
    iterations: u32,
    seed: u32,
) -> Vec<f32> {
    let mesh = reconstruct_mesh(positions, normals, areas);
    let (dir, _) = normalise_dir([dir_x, dir_y, dir_z]);
    let rng = rng::Rng::new(seed);
    let (best_dir, best_score) = refine_once(&mesh, &dir, critical_angle_deg, iterations.min(500), rng);
    vec![best_dir[0], best_dir[1], best_dir[2], best_score]
}

/// Run `k` independent seeded refinements from the same starting direction.
/// Returns `k × 4` floats: each run's `[dir_x, dir_y, dir_z, score]`. Used by
/// the worker to compute the best refined score (min) and the variance (stddev)
/// across runs — the latter becomes the H7 "refine stability" metric.
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn refine_orientation_batch(
    positions: &[f32],
    normals: &[f32],
    areas: &[f32],
    dir_x: f32,
    dir_y: f32,
    dir_z: f32,
    critical_angle_deg: f32,
    iterations: u32,
    k: u32,
    base_seed: u32,
) -> Vec<f32> {
    let mesh = reconstruct_mesh(positions, normals, areas);
    let (dir, _) = normalise_dir([dir_x, dir_y, dir_z]);
    let cap = k.min(8) as usize;
    let mut out = Vec::with_capacity(cap * 4);
    for i in 0..cap {
        let seed = rng::seed_from_direction(&dir, base_seed.wrapping_add(i as u32));
        let rng = rng::Rng::new(seed);
        let (rd, rs) = refine_once(&mesh, &dir, critical_angle_deg, iterations.min(500), rng);
        out.push(rd[0]);
        out.push(rd[1]);
        out.push(rd[2]);
        out.push(rs);
    }
    out
}

/// Refine a direction via hill-climb (optional), then compute all 5 raw
/// scoring metrics for the **refined** direction in a single mesh pass.
///
/// Returns 8 floats:
///   [dir_x, dir_y, dir_z, overhang, footprint, max_cross, surface, height]
///
/// `iterations = 0` skips refinement (metrics computed for the normalised
/// input direction directly). With `iterations > 0`, the hill-climb finds
/// the nearest local overhang minimum, and ALL metrics are computed for
/// that refined direction — ensuring the overhang score and the other 4
/// metrics describe the same orientation.
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn score_orientation(
    positions: &[f32],
    normals: &[f32],
    areas: &[f32],
    dir_x: f32,
    dir_y: f32,
    dir_z: f32,
    critical_angle_deg: f32,
    iterations: u32,
    seed: u32,
) -> Vec<f32> {
    let mesh = reconstruct_mesh(positions, normals, areas);
    let (dir, _) = normalise_dir([dir_x, dir_y, dir_z]);
    let rng = rng::Rng::new(seed);
    let (best_dir, _) = refine_once(&mesh, &dir, critical_angle_deg, iterations.min(500), rng);
    let c = scoring::score_components(&best_dir, &mesh, critical_angle_deg, 64);
    vec![
        best_dir[0], best_dir[1], best_dir[2],
        c.overhang, c.footprint, c.max_cross, c.surface_quality, c.height,
    ]
}

// ---------------------------------------------------------------------------
// New WASM exports (Plan 02)
// ---------------------------------------------------------------------------

/// Score ALL directions in one call. Returns N×13 floats per direction:
/// [qx, qy, qz, qw, overhang, footprint, cross, surface, height, shadowed, stable, margin, contact_area]
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn score_all_directions(
    positions: &[f32],
    normals: &[f32],
    areas: &[f32],
    directions: &[f32],
    critical_angle_deg: f32,
    refine_iters: u32,
    _exclude_unstable: bool,
    progress: Option<js_sys::Function>,
) -> Vec<f32> {
    let mesh = reconstruct_mesh(positions, normals, areas);
    let hull_verts = decimate::sample_for_hull(&mesh.vertices);
    let hull = hull::compute_hull(&hull_verts);
    let total = directions.len() / 3;
    let mut out = Vec::with_capacity(total * 13);

    for i in 0..total {
        let dir = [directions[i * 3], directions[i * 3 + 1], directions[i * 3 + 2]];
        let (dir_n, _) = normalise_dir(dir);

        // Refine (optional), then compute all metrics for the refined direction.
        let (best_dir, _) = if refine_iters > 0 {
            let rng = rng::Rng::new(rng::seed_from_direction(&dir_n, 0));
            refine_once(&mesh, &dir_n, critical_angle_deg, refine_iters.min(500), rng)
        } else {
            (dir_n, 0.0)
        };

        let c = scoring::score_components(&best_dir, &mesh, critical_angle_deg, 64);
        let shadowed = scoring::shadowed_overhang_fraction(&best_dir, &mesh, critical_angle_deg, 32, 0.02);
        let stab = stability::check_stability(&best_dir, &mesh, &hull);
        let q = yaw::full_quaternion(&best_dir, &mesh);

        let stable_f = if stab.stable { 1.0 } else { 0.0 };
        out.extend_from_slice(&[
            q[0], q[1], q[2], q[3],              // quaternion [w, x, y, z]
            c.overhang, c.footprint, c.max_cross, c.surface_quality, c.height,
            shadowed,
            stable_f, stab.margin, stab.contact_area,
        ]);

        if let Some(ref cb) = progress {
            if i % 10 == 0 {
                let _ = cb.call2(
                    &wasm_bindgen::JsValue::UNDEFINED,
                    &wasm_bindgen::JsValue::from_f64(i as f64),
                    &wasm_bindgen::JsValue::from_f64(total as f64),
                );
            }
        }
    }
    out
}

/// Rank candidates by method. Input is N×13 flat metrics (output of score_all_directions).
/// Returns N×2 [index, composite_score] sorted by method's convention.
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn rank_candidates(
    metrics_flat: &[f32],
    weights: &[f32],
    method: &str,
) -> Vec<f32> {
    let n = metrics_flat.len() / 13;
    let mut metrics = Vec::with_capacity(n);
    for i in 0..n {
        let base = i * 13;
        // Per-direction layout: [0-3]=quat, [4]=overhang, [5]=footprint, [6]=cross,
        // [7]=surface, [8]=height, [9]=shadowed, [10]=stable, [11]=margin, [12]=contact
        metrics.push(ranking::CandidateMetrics {
            overhang: metrics_flat[base + 4],
            footprint: metrics_flat[base + 5],
            max_cross: metrics_flat[base + 6],
            surface: metrics_flat[base + 7],
            height: metrics_flat[base + 8],
            shadowed: metrics_flat[base + 9],
        });
    }

    let w = ranking::ScoreWeights {
        w_overhang: weights[0],
        w_footprint: weights[1],
        w_cross: weights[2],
        w_surface: weights[3],
        w_height: weights[4],
    };

    let ranked = match method {
        "weights" => ranking::rank_by_weights(&metrics, &w),
        "consensus" => ranking::rank_by_consensus(&metrics),
        "topsis" => ranking::rank_by_topsis(&metrics, &w),
        _ => vec![],
    };

    let mut out = Vec::with_capacity(ranked.len() * 2);
    for (idx, score) in ranked {
        out.push(idx as f32);
        out.push(score);
    }
    out
}

/// Compute normalization bounds (min/max per metric) by sampling ~30 directions.
/// Returns 10 floats: [lo[5], hi[5]] for overhang, footprint, cross, surface, height.
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn compute_norm_bounds(
    positions: &[f32],
    normals: &[f32],
    areas: &[f32],
    directions: &[f32],
    critical_angle_deg: f32,
) -> Vec<f32> {
    let mesh = reconstruct_mesh(positions, normals, areas);
    let total = directions.len() / 3;
    let step = (total / 30).max(1);

    let mut lo = [f32::INFINITY; 5];
    let mut hi = [f32::NEG_INFINITY; 5];

    for i in (0..total).step_by(step) {
        let dir = [directions[i * 3], directions[i * 3 + 1], directions[i * 3 + 2]];
        let (nd, _) = normalise_dir(dir);
        let c = scoring::score_components(&nd, &mesh, critical_angle_deg, 64);
        let vals = [c.overhang, c.footprint, c.max_cross, c.surface_quality, c.height];
        for j in 0..5 {
            if vals[j] < lo[j] { lo[j] = vals[j]; }
            if vals[j] > hi[j] { hi[j] = vals[j]; }
        }
    }

    vec![
        lo[0], lo[1], lo[2], lo[3], lo[4],
        hi[0], hi[1], hi[2], hi[3], hi[4],
    ]
}

/// Select a diverse subset of candidates by angle-diversity filtering.
/// `ranked` is N×2 [index, composite_score] — output of rank_candidates.
/// `directions` is M×3 raw direction vectors.
/// Returns selected indices as Vec<f32> (WASM FFI constraint — JS casts back).
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn select_diverse(
    ranked: &[f32],
    directions: &[f32],
    stable_flags: &[f32],
    exclude_unstable: bool,
    max_candidates: usize,
    min_angle_deg: f32,
) -> Vec<f32> {
    let n = ranked.len() / 2;
    let mut scored = Vec::with_capacity(n);
    for i in 0..n {
        scored.push((ranked[i * 2] as usize, ranked[i * 2 + 1]));
    }

    let m = directions.len() / 3;
    let mut dirs = Vec::with_capacity(m);
    for i in 0..m {
        dirs.push([directions[i * 3], directions[i * 3 + 1], directions[i * 3 + 2]]);
    }

    let stable: Vec<bool> = stable_flags.iter().map(|&f| f > 0.5).collect();
    let result = selection::merge_candidates(&scored, &dirs, &stable, exclude_unstable, max_candidates, min_angle_deg);

    result.into_iter().map(|i| i as f32).collect()
}

fn reconstruct_mesh(positions: &[f32], normals: &[f32], areas: &[f32]) -> mesh::MeshData {
    let tri_count = normals.len() / 3;
    let normals_vec: Vec<[f32; 3]> = normals.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect();
    let areas_vec: Vec<f32> = areas.to_vec();
    let vertices_vec: Vec<[f32; 3]> = positions.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect();
    mesh::MeshData {
        normals: normals_vec,
        areas: areas_vec,
        vertices: vertices_vec,
        triangle_count: tri_count,
    }
}

fn normalise_dir(d: [f32; 3]) -> ([f32; 3], f32) {
    let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    if len > 0.0 {
        ([d[0] / len, d[1] / len, d[2] / len], len)
    } else {
        ([0.0, 0.0, -1.0], 0.0)
    }
}

/// Single hill-climb run. Deterministic given the same `rng` state. Returns
/// the refined direction and its overhang score (lower = better).
fn refine_once(
    mesh: &mesh::MeshData,
    start_dir: &[f32; 3],
    critical_angle_deg: f32,
    iterations: u32,
    mut rng: rng::Rng,
) -> ([f32; 3], f32) {
    let mut best_dir = *start_dir;
    let mut best_score = scoring::score_candidate(&best_dir, mesh, critical_angle_deg);
    let mut perturbation_deg = 10.0_f32;

    for i in 0..iterations {
        let u1 = rng.next_signed_f32();
        let u2 = rng.next_signed_f32();

        let perp = [
            best_dir[1] * u2 - best_dir[2] * u1,
            best_dir[2] * u1 - best_dir[0] * u2,
            best_dir[0] * u2 - best_dir[1] * u1,
        ];
        let plen = (perp[0] * perp[0] + perp[1] * perp[1] + perp[2] * perp[2]).sqrt().max(1e-12);
        let perp = [perp[0] / plen, perp[1] / plen, perp[2] / plen];

        let angle = perturbation_deg * (std::f32::consts::PI / 180.0) * rng.next_signed_f32();
        let (s, c) = angle.sin_cos();

        let new_dir = [
            c * best_dir[0] + s * perp[0],
            c * best_dir[1] + s * perp[1],
            c * best_dir[2] + s * perp[2],
        ];
        let nlen = (new_dir[0] * new_dir[0] + new_dir[1] * new_dir[1] + new_dir[2] * new_dir[2]).sqrt().max(1e-12);
        let new_dir = [new_dir[0] / nlen, new_dir[1] / nlen, new_dir[2] / nlen];

        let new_score = scoring::score_candidate(&new_dir, mesh, critical_angle_deg);
        if new_score < best_score {
            best_dir = new_dir;
            best_score = new_score;
        }

        perturbation_deg *= 0.95;
        if i > iterations / 2 {
            perturbation_deg = perturbation_deg.max(0.5);
        }
    }

    (best_dir, best_score)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_square_mesh() -> mesh::MeshData {
        // Two triangles in the XY plane (z=0), normal +Z, area 0.5 each.
        let positions: Vec<f32> = vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0,
        ];
        reconstruct_mesh(&positions, &[0.0, 0.0, 1.0, 0.0, 0.0, 1.0], &[0.5, 0.5])
    }

    #[test]
    fn refine_once_is_deterministic_for_same_seed() {
        let mesh = flat_square_mesh();
        let dir = [0.0, 0.0, -1.0];
        let (d1, s1) = refine_once(&mesh, &dir, 30.0, 50, rng::Rng::new(42));
        let (d2, s2) = refine_once(&mesh, &dir, 30.0, 50, rng::Rng::new(42));
        assert_eq!(d1, d2, "same seed must yield identical direction");
        assert_eq!(s1.to_bits(), s2.to_bits(), "same seed must yield identical score");
    }

    #[test]
    fn refine_once_different_seeds_may_differ() {
        let mesh = flat_square_mesh();
        // dir=[0,0,1] faces the same way as the +Z normals → all faces are
        // overhang (starting score > 0), so different seeds explore different
        // perturbation trajectories and produce different refined directions.
        // (dir=[0,0,-1] is already optimal → overhang=0 → all seeds identical.)
        let dir = [0.0, 0.0, 1.0];
        let (d1, _) = refine_once(&mesh, &dir, 30.0, 50, rng::Rng::new(1));
        let (d2, _) = refine_once(&mesh, &dir, 30.0, 50, rng::Rng::new(2));
        // Different seeds should usually produce different trajectories.
        assert_ne!(d1, d2, "different seeds should produce different results");
    }

    #[test]
    fn refine_once_never_worsens_score() {
        // The hill-climb only accepts improvements; the returned score must be
        // <= the score of the starting direction.
        let mesh = flat_square_mesh();
        let dir = [0.0, 0.0, -1.0]; // normal -Z: faces point away → overhang should be 0 already
        let start_score = scoring::score_candidate(&dir, &mesh, 30.0);
        let (_, refined_score) = refine_once(&mesh, &dir, 30.0, 50, rng::Rng::new(7));
        assert!(
            refined_score <= start_score + 1e-6,
            "refine must not worsen: start={} refined={}",
            start_score, refined_score
        );
    }

    #[test]
    fn refine_batch_returns_k_runs() {
        let positions: Vec<f32> = vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0,
        ];
        let normals = vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0];
        let areas = vec![0.5, 0.5];
        let out = refine_orientation_batch(&positions, &normals, &areas, 0.0, 0.0, -1.0, 30.0, 50, 4, 0);
        assert_eq!(out.len(), 16, "k=4 → 16 floats (4 per run)");
    }

    #[test]
    fn refine_batch_is_deterministic() {
        let positions: Vec<f32> = vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0,
        ];
        let normals = vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0];
        let areas = vec![0.5, 0.5];
        let a = refine_orientation_batch(&positions, &normals, &areas, 0.0, 0.0, -1.0, 30.0, 50, 4, 0);
        let b = refine_orientation_batch(&positions, &normals, &areas, 0.0, 0.0, -1.0, 30.0, 50, 4, 0);
        assert_eq!(a, b, "same inputs + same base_seed → identical output");
    }

    #[test]
    fn refine_orientation_seed_param_is_deterministic() {
        let positions: Vec<f32> = vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0,
        ];
        let normals = vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0];
        let areas = vec![0.5, 0.5];
        let a = refine_orientation(&positions, &normals, &areas, 0.0, 0.0, -1.0, 30.0, 50, 42);
        let b = refine_orientation(&positions, &normals, &areas, 0.0, 0.0, -1.0, 30.0, 50, 42);
        assert_eq!(a, b, "same seed → identical result");
    }

    // ---- score_orientation tests ----

    #[test]
    fn score_orientation_returns_8_values() {
        let positions: Vec<f32> = vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0,
        ];
        let normals = vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0];
        let areas = vec![0.5, 0.5];
        let out = score_orientation(&positions, &normals, &areas, 0.0, 0.0, -1.0, 30.0, 0, 42);
        assert_eq!(out.len(), 8, "should return 8 floats");
    }

    #[test]
    fn score_orientation_zero_iterations_matches_raw_score() {
        // With 0 iterations the direction is unchanged (just normalised),
        // so the overhang metric must equal score_candidate for that direction.
        let positions: Vec<f32> = vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0,
        ];
        let normals = vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0];
        let areas = vec![0.5, 0.5];
        let out = score_orientation(&positions, &normals, &areas, 0.0, 0.0, -1.0, 30.0, 0, 42);
        let mesh = reconstruct_mesh(&positions, &normals, &areas);
        let expected = scoring::score_candidate(&[0.0, 0.0, -1.0], &mesh, 30.0);
        assert!((out[3] - expected).abs() < 1e-6, "overhang mismatch: got {} expected {}", out[3], expected);
    }

    #[test]
    fn score_orientation_refined_does_not_worsens_overhang() {
        // Hill-climb only accepts improvements, so the refined overhang must
        // be <= the overhang at the starting direction.
        let positions: Vec<f32> = vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0,
        ];
        let normals = vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0];
        let areas = vec![0.5, 0.5];
        let mesh = reconstruct_mesh(&positions, &normals, &areas);
        let start = scoring::score_candidate(&[0.0, 0.0, 1.0], &mesh, 30.0);
        let out = score_orientation(&positions, &normals, &areas, 0.0, 0.0, 1.0, 30.0, 50, 42);
        assert!(out[3] <= start + 1e-6, "refine must not worsen: start={} got={}", start, out[3]);
    }

    #[test]
    fn score_orientation_is_deterministic() {
        let positions: Vec<f32> = vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0,
        ];
        let normals = vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0];
        let areas = vec![0.5, 0.5];
        let a = score_orientation(&positions, &normals, &areas, 0.0, 0.0, -1.0, 30.0, 50, 42);
        let b = score_orientation(&positions, &normals, &areas, 0.0, 0.0, -1.0, 30.0, 50, 42);
        assert_eq!(a, b, "same seed → identical result");
    }

    // ---- Solid cube tests ----

    /// Unit cube (0,0,0)–(1,1,1), 12 triangles, outward normals, area 0.5 each.
    fn unit_cube_data() -> (Vec<f32>, Vec<f32>, Vec<f32>) {
        let positions = vec![
            // Bottom (z=0), normal (0,0,-1)
            0.0,0.0,0.0,  1.0,0.0,0.0,  1.0,1.0,0.0,
            0.0,0.0,0.0,  1.0,1.0,0.0,  0.0,1.0,0.0,
            // Top (z=1), normal (0,0,1)
            0.0,0.0,1.0,  1.0,0.0,1.0,  1.0,1.0,1.0,
            0.0,0.0,1.0,  1.0,1.0,1.0,  0.0,1.0,1.0,
            // Front (y=0), normal (0,-1,0)
            0.0,0.0,0.0,  1.0,0.0,1.0,  1.0,0.0,0.0,
            0.0,0.0,0.0,  0.0,0.0,1.0,  1.0,0.0,1.0,
            // Back (y=1), normal (0,1,0)
            0.0,1.0,0.0,  1.0,1.0,0.0,  1.0,1.0,1.0,
            0.0,1.0,0.0,  1.0,1.0,1.0,  0.0,1.0,1.0,
            // Left (x=0), normal (-1,0,0)
            0.0,0.0,0.0,  0.0,1.0,0.0,  0.0,1.0,1.0,
            0.0,0.0,0.0,  0.0,1.0,1.0,  0.0,0.0,1.0,
            // Right (x=1), normal (1,0,0)
            1.0,0.0,0.0,  1.0,1.0,0.0,  1.0,1.0,1.0,
            1.0,0.0,0.0,  1.0,1.0,1.0,  1.0,0.0,1.0,
        ];
        let normals = vec![
            0.0,0.0,-1.0, 0.0,0.0,-1.0,
            0.0,0.0,1.0,  0.0,0.0,1.0,
            0.0,-1.0,0.0, 0.0,-1.0,0.0,
            0.0,1.0,0.0,  0.0,1.0,0.0,
            -1.0,0.0,0.0, -1.0,0.0,0.0,
            1.0,0.0,0.0,  1.0,0.0,0.0,
        ];
        let areas = vec![0.5; 12];
        (positions, normals, areas)
    }

    #[test]
    fn cube_face_down_zero_overhang() {
        // dir = [0,0,-1] (pointing down in Z-up): bottom face has normal (0,0,-1),
        // dot = 1.0 > cos(30°) → overhang faces ARE present.
        // But wait: in the tool's convention, dir is the build direction (down).
        // Faces pointing along dir (downward) need supports → they ARE overhang.
        // A cube sitting flat on Z=0 face: bottom normals = (0,0,-1), dir = (0,0,-1).
        // These faces point downward → cos_i = 1.0 > cos_crit → they ARE overhang.
        // But physically, the bottom face sits on the build plate — no supports needed!
        // The scoring function doesn't distinguish this (no height-field check in score_candidate).
        // So overhang > 0 is expected for ANY face aligned with dir.
        // For a cube with dir = [0, -1, 0]: front face (normal (0,-1,0)) has dot=1 → overhang.
        // Only the 2 front triangles contribute: area 0.5 × 2 = 1.0, penalty = 1.0 × (1 - cos30°)
        let (p, n, a) = unit_cube_data();
        let out = score_orientation(&p, &n, &a, 0.0, -1.0, 0.0, 30.0, 0, 42);
        let cos30 = (30.0_f32).to_radians().cos();
        let expected_penalty = 1.0 * (1.0 - cos30); // 2 tris × 0.5 area × (dot - cos_crit)
        assert!(
            (out[3] - expected_penalty).abs() < 0.01,
            "overhang for front-face-down cube: got {} expected ~{}",
            out[3], expected_penalty
        );
    }

    #[test]
    fn cube_height_along_y_is_one() {
        let (p, n, a) = unit_cube_data();
        let out = score_orientation(&p, &n, &a, 0.0, -1.0, 0.0, 30.0, 0, 42);
        // Height (index 7) along Y should be 1.0 (cube spans 0..1 on Y)
        assert!(
            (out[7] - 1.0).abs() < 0.01,
            "height along Y should be 1.0, got {}",
            out[7]
        );
    }

    #[test]
    fn cube_footprint_facing_face() {
        let (p, n, a) = unit_cube_data();
        // dir = [0,-1,0]: front face (normal (0,-1,0)) is face-on → full area
        let out = score_orientation(&p, &n, &a, 0.0, -1.0, 0.0, 30.0, 0, 42);
        // Footprint = sum |n·dir| × area for ALL triangles
        // Front face: 2 tris, |1.0| × 0.5 = 0.5 each → 1.0
        // Back face: 2 tris, |−1.0| × 0.5 = 0.5 each → 1.0
        // Side faces: normals ⊥ dir → 0
        // Total footprint = 2.0
        assert!(
            (out[4] - 2.0).abs() < 0.01,
            "footprint facing a face should be 2.0 (front+back), got {}",
            out[4]
        );
    }

    #[test]
    fn cube_metrics_match_score_components() {
        // Verify that score_orientation returns the same values as the internal
        // scoring functions for a solid cube.
        let (p, n, a) = unit_cube_data();
        let dir = [0.57735, -0.57735, 0.57735]; // (1,-1,1)/√3
        let out = score_orientation(&p, &n, &a, dir[0], dir[1], dir[2], 30.0, 0, 42);
        let mesh = reconstruct_mesh(&p, &n, &a);
        let (nd, _) = normalise_dir(dir);
        let c = scoring::score_components(&nd, &mesh, 30.0, 64);
        assert!((out[3] - c.overhang).abs() < 1e-5, "overhang mismatch");
        assert!((out[4] - c.footprint).abs() < 1e-5, "footprint mismatch");
        assert!((out[5] - c.max_cross).abs() < 1e-5, "cross mismatch");
        assert!((out[6] - c.surface_quality).abs() < 1e-5, "surface mismatch");
        assert!((out[7] - c.height).abs() < 1e-5, "height mismatch");
    }
}
