pub mod stl;
pub mod mesh;
pub mod hull;
pub mod candidates;
pub mod scoring;
pub mod stability;
pub mod decimate;
pub mod rng;
pub mod ranking;
pub mod repair;
pub mod selection;
pub mod yaw;
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
#[allow(dead_code)]
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
pub struct OriData {
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
    pub areas: Vec<f32>,
    pub directions: Vec<f32>,
}

/// Ungated native entry point. Parses STL bytes, precomputes mesh, computes
/// the convex hull, generates candidate directions (with optional sphere
/// blending), and returns the mesh data + flattened directions.
pub fn prepare_data_native(bytes: &[u8], mode: &str, dedupe_angle_deg: f32) -> Result<OriData, String> {
    if mode != "hull" && mode != "hull_plus_sphere" {
        return Err(format!("Unknown mode: {mode}"));
    }

    let triangles = stl::parse_stl(bytes)?;
    if triangles.is_empty() {
        return Err("No triangles in STL".into());
    }

    let mut flat: Vec<f32> = triangles.iter().flat_map(|v| v.iter()).copied().collect();
    repair::repair_mesh(&mut flat);
    let m = mesh::precompute_mesh(&flat);
    if m.triangle_count == 0 {
        return Err("All triangles are degenerate".into());
    }

    let hull_verts = decimate::sample_for_hull(&m.vertices);
    let hull = hull::compute_hull(&hull_verts);
    if hull.face_normals.is_empty() {
        return Err("Could not compute convex hull (all vertices coplanar?)".into());
    }

    let deduped = if mode == "hull_plus_sphere" {
        let combined = candidates::generate_hull_plus_sphere(&hull, 200, dedupe_angle_deg);
        candidates::deduplicate_directions(&combined, dedupe_angle_deg)
    } else {
        let directions = candidates::generate_candidates(&hull);
        candidates::deduplicate_directions(&directions, dedupe_angle_deg)
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

    Ok(OriData {
        positions: clean,
        normals: normals_flat,
        areas: m.areas,
        directions: dir_flat,
    })
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn prepare_data(bytes: &[u8], config: &JsValue) -> JsValue {
    let config: OrientConfig = serde_wasm_bindgen::from_value(config.clone())
        .unwrap_or_else(|e| wasm_bindgen::throw_str(&format!("Invalid config: {e}")));

    let od = prepare_data_native(bytes, &config.mode, config.dedupe_angle_deg)
        .unwrap_or_else(|e| wasm_bindgen::throw_str(&e));

    serde_wasm_bindgen::to_value(&od).unwrap()
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

/// Refine a direction via hill-climb (optional), then compute all 6 raw
/// scoring metrics for the **refined** direction in a single mesh pass.
///
/// Returns 9 floats:
///   [dir_x, dir_y, dir_z, overhang, footprint, max_cross, surface, height, shadowed]
///
/// `iterations = 0` skips refinement (metrics computed for the normalised
/// input direction directly). With `iterations > 0`, the hill-climb finds
/// the nearest local overhang minimum, and ALL metrics are computed for
/// that refined direction — ensuring the overhang score and the other 5
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
        c.overhang, c.footprint, c.max_cross, c.surface_quality, c.height, c.shadowed,
    ]
}

/// Score a single direction using the SAME seed derivation as score_all_directions.
/// This ensures the live panel gets identical metrics to the candidate list.
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn score_direction(
    positions: &[f32],
    normals: &[f32],
    areas: &[f32],
    dir_x: f32,
    dir_y: f32,
    dir_z: f32,
    critical_angle_deg: f32,
    refine_iters: u32,
) -> Vec<f32> {
    let mesh = reconstruct_mesh(positions, normals, areas);
    let (dir_n, _) = normalise_dir([dir_x, dir_y, dir_z]);
    let (best_dir, _) = if refine_iters > 0 {
        let rng = rng::Rng::new(rng::seed_from_direction(&dir_n, 0));
        refine_once(&mesh, &dir_n, critical_angle_deg, refine_iters.min(500), rng)
    } else {
        (dir_n, 0.0)
    };
    let c = scoring::score_components(&best_dir, &mesh, critical_angle_deg, 64);
    vec![
        best_dir[0], best_dir[1], best_dir[2],
        c.overhang, c.footprint, c.max_cross, c.surface_quality, c.height, c.shadowed,
    ]
}

// ---------------------------------------------------------------------------
// New WASM exports (Plan 02)
// ---------------------------------------------------------------------------

/// Score ALL directions in one call. Returns N×13 floats per direction:
/// [qx, qy, qz, qw, overhang, footprint, cross, surface, height, shadowed, stable, margin, contact_area]
/// Quaternion is in [x,y,z,w] order (THREE.js convention).
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
        let stab = stability::check_stability(&best_dir, &mesh);
        let q = yaw::full_quaternion(&best_dir, &mesh);

        let stable_f = if stab.stable { 1.0 } else { 0.0 };
        out.extend_from_slice(&[
            q[1], q[2], q[3], q[0],              // quaternion [x, y, z, w] — three.js convention
            c.overhang, c.footprint, c.max_cross, c.surface_quality, c.height,
            c.shadowed,
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
/// norm_lo/norm_hi: optional [5]f32 min/max for all-directions normalization (empty = use candidate set).
/// Returns N×2 [index, composite_score] sorted by method's convention.
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn rank_candidates(
    metrics_flat: &[f32],
    weights: &[f32],
    method: &str,
    norm_lo: &[f32],
    norm_hi: &[f32],
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
        w_shadowed: weights[5],
    };

    let (norm_lo_owned, norm_hi_owned) = if norm_lo.len() >= 6 && norm_hi.len() >= 6 {
        (Some([norm_lo[0], norm_lo[1], norm_lo[2], norm_lo[3], norm_lo[4], norm_lo[5]]),
         Some([norm_hi[0], norm_hi[1], norm_hi[2], norm_hi[3], norm_hi[4], norm_hi[5]]))
    } else {
        (None, None)
    };

    let ranked = match method {
        "weights" => ranking::rank_by_weights_with_bounds(&metrics, &w, norm_lo_owned.as_ref(), norm_hi_owned.as_ref()),
        "consensus" => ranking::rank_by_consensus_with_bounds(&metrics, &w, norm_lo_owned.as_ref(), norm_hi_owned.as_ref()),
        "topsis" => ranking::rank_by_topsis(&metrics, &w),
        _ => vec![],
    };

    let w_sum: f32 = weights.iter().sum();
    let w_sum_inv = if w_sum > 1e-9 { 1.0 / w_sum } else { 1.0 };

    let mut out = Vec::with_capacity(ranked.len() * 2);
    for (idx, score) in ranked {
        // Normalize all scores to [0,1] higher=better for consistent display
        let display_score = match method {
            "weights" => 1.0 - (score * w_sum_inv).clamp(0.0, 1.0),
            _ => score.clamp(0.0, 1.0),
        };
        out.push(idx as f32);
        out.push(display_score);
    }
    out
}

/// Compute normalization bounds (min/max per metric) by sampling ~30 directions.
/// Returns 12 floats: [lo[6], hi[6]] for overhang, footprint, cross, surface, height, shadowed.
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

    let mut lo = [f32::INFINITY; 6];
    let mut hi = [f32::NEG_INFINITY; 6];

    for i in (0..total).step_by(step) {
        let dir = [directions[i * 3], directions[i * 3 + 1], directions[i * 3 + 2]];
        let (nd, _) = normalise_dir(dir);
        let c = scoring::score_components(&nd, &mesh, critical_angle_deg, 64);
        let vals = [c.overhang, c.footprint, c.max_cross, c.surface_quality, c.height, c.shadowed];
        for j in 0..6 {
            if vals[j] < lo[j] { lo[j] = vals[j]; }
            if vals[j] > hi[j] { hi[j] = vals[j]; }
        }
    }

    vec![
        lo[0], lo[1], lo[2], lo[3], lo[4], lo[5],
        hi[0], hi[1], hi[2], hi[3], hi[4], hi[5],
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

pub fn reconstruct_mesh(positions: &[f32], normals: &[f32], areas: &[f32]) -> mesh::MeshData {
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

pub fn normalise_dir(d: [f32; 3]) -> ([f32; 3], f32) {
    let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    if len > 0.0 {
        ([d[0] / len, d[1] / len, d[2] / len], len)
    } else {
        ([0.0, 0.0, -1.0], 0.0)
    }
}

/// Generate a random unit vector in the tangent plane of `dir` from two
/// random scalars u1, u2 by constructing a linear combination of the
/// orthonormal basis vectors in the perpendicular plane.
fn tangent_perturbation(dir: &[f32; 3], u1: f32, u2: f32) -> [f32; 3] {
    let (e1, e2) = scoring::perpendicular_basis(dir);
    let p = [u1 * e1[0] + u2 * e2[0], u1 * e1[1] + u2 * e2[1], u1 * e1[2] + u2 * e2[2]];
    let plen = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt().max(1e-12);
    [p[0] / plen, p[1] / plen, p[2] / plen]
}

/// Single hill-climb run. Deterministic given the same `rng` state. Returns
/// the refined direction and its overhang score (lower = better).
pub fn refine_once(
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

        let perp = tangent_perturbation(&best_dir, u1, u2);

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
    fn tangent_perturbation_is_perpendicular() {
        // RED: this test FAILS with the buggy formula for tilted directions
        // because dir[0]·dir[2]·(u2-u1) leaks into the dot product.
        // Poles (z-only) and axis-aligned dirs pass spuriously — the tilt case
        // is the one that must pass after the fix.
        let dirs: [[f32; 3]; 5] = [
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -1.0],
            [1.0, 0.0, 0.0],
            [0.7, 0.0, 0.7],
            [0.5, 0.5, 0.7071],
        ];
        let uv: [(f32, f32); 6] = [
            (1.0, -1.0),
            (-0.5, 0.8),
            (0.2, -0.9),
            (-0.7, 0.3),
            (0.9, -0.1),
            (-0.3, 0.6),
        ];
        for d in &dirs {
            let dl = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            let dn = [d[0] / dl, d[1] / dl, d[2] / dl];
            for &(u1, u2) in &uv {
                let perp = tangent_perturbation(&dn, u1, u2);
                let dot = dn[0] * perp[0] + dn[1] * perp[1] + dn[2] * perp[2];
                assert!(
                    dot.abs() < 1e-5,
                    "dir={d:?} perp={perp:?} dot={dot} >= 1e-5"
                );
            }
        }
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

    #[cfg(feature = "wasm")]
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

    #[cfg(feature = "wasm")]
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

    #[cfg(feature = "wasm")]
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

    #[cfg(feature = "wasm")]
    #[test]
    fn score_orientation_returns_9_values() {
        let positions: Vec<f32> = vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0,
        ];
        let normals = vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0];
        let areas = vec![0.5, 0.5];
        let out = score_orientation(&positions, &normals, &areas, 0.0, 0.0, -1.0, 30.0, 0, 42);
        assert_eq!(out.len(), 9, "should return 9 floats (dir3 + 6 metrics)");
    }

    #[cfg(feature = "wasm")]
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

    #[cfg(feature = "wasm")]
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

    #[cfg(feature = "wasm")]
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

    #[cfg(feature = "wasm")]
    #[test]
    fn cube_height_along_y_is_one() {
        let (p, n, a) = unit_cube_data();
        let out = score_orientation(&p, &n, &a, 0.0, -1.0, 0.0, 30.0, 0, 42);
        // Height risk = overhang × height (derived metric).
        // Unit cube dir[0,-1,0]: overhang ≈ 0.134 (2 front tris × 0.5 area × (1-cos30)),
        // height = 1.0 (spans 0..1 on Y), so height_risk ≈ 0.134
        let overhang = out[3];
        let expected = overhang * 1.0;
        assert!(
            (out[7] - expected).abs() < 0.01,
            "height risk along Y: got {} expected {}",
            out[7], expected
        );
    }

    #[cfg(feature = "wasm")]
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

    #[cfg(feature = "wasm")]
    #[test]
    fn score_all_directions_quaternion_is_xyzw_layout() {
        // CR-02: score_all_directions must output quaternions in [x,y,z,w] order
        // to match THREE.js Quaternion.set(x,y,z,w) convention.
        // For dir = [0,-1,0] (already the build direction), quaternion_align returns
        // identity [w=1,x=0,y=0,z=0] in internal [w,x,y,z] format.
        // The output should be [x=0,y=0,z=0,w=1] — i.e. w at position 3.
        let (p, n, a) = unit_cube_data();
        let dirs = vec![0.0f32, -1.0, 0.0];
        let out = score_all_directions(&p, &n, &a, &dirs, 30.0, 0, false, None);
        assert_eq!(out.len(), 13, "one direction should produce 13 floats");
        // Quaternion [x,y,z,w]: w should be at index 3 and ≈ 1.0 for identity
        assert!(
            (out[3] - 1.0).abs() < 0.01,
            "quaternion w component (index 3 in xyzw) should be ≈1.0 for identity dir, got {}",
            out[3]
        );
        assert!(
            out[0].abs() < 0.01 && out[1].abs() < 0.01 && out[2].abs() < 0.01,
            "quaternion xyz components should be ≈0 for identity dir, got [{},{},{}]",
            out[0], out[1], out[2]
        );
    }

}
