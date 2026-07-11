mod stl;
mod mesh;
mod hull;
mod candidates;
mod scoring;
mod stability;
mod decimate;
#[cfg(test)]
mod harness;

use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};

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
) -> Vec<f32> {
    let tri_count = normals.len() / 3;
    let normals_vec: Vec<[f32; 3]> = normals.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect();
    let areas_vec: Vec<f32> = areas.to_vec();
    let vertices_vec: Vec<[f32; 3]> = positions.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect();
    let mesh = mesh::MeshData {
        normals: normals_vec,
        areas: areas_vec,
        vertices: vertices_vec,
        triangle_count: tri_count,
    };

    let mut best_dir = [dir_x, dir_y, dir_z];
    let len = (best_dir[0] * best_dir[0] + best_dir[1] * best_dir[1] + best_dir[2] * best_dir[2]).sqrt();
    if len > 0.0 {
        best_dir = [best_dir[0] / len, best_dir[1] / len, best_dir[2] / len];
    } else {
        best_dir = [0.0, 0.0, -1.0];
    }
    let mut best_score = scoring::score_candidate(&best_dir, &mesh, critical_angle_deg);
    let mut perturbation_deg = 10.0_f32;

    for i in 0..iterations.min(500) {
        let u1 = js_sys::Math::random() as f32 * 2.0 - 1.0;
        let u2 = js_sys::Math::random() as f32 * 2.0 - 1.0;

        let perp = [
            best_dir[1] * u2 - best_dir[2] * u1,
            best_dir[2] * u1 - best_dir[0] * u2,
            best_dir[0] * u2 - best_dir[1] * u1,
        ];
        let plen = (perp[0] * perp[0] + perp[1] * perp[1] + perp[2] * perp[2]).sqrt().max(1e-12);
        let perp = [perp[0] / plen, perp[1] / plen, perp[2] / plen];

        let angle = perturbation_deg * (std::f32::consts::PI / 180.0) * (js_sys::Math::random() as f32 * 2.0 - 1.0);
        let (s, c) = angle.sin_cos();

        let new_dir = [
            c * best_dir[0] + s * perp[0],
            c * best_dir[1] + s * perp[1],
            c * best_dir[2] + s * perp[2],
        ];
        let nlen = (new_dir[0] * new_dir[0] + new_dir[1] * new_dir[1] + new_dir[2] * new_dir[2]).sqrt().max(1e-12);
        let new_dir = [new_dir[0] / nlen, new_dir[1] / nlen, new_dir[2] / nlen];

        let new_score = scoring::score_candidate(&new_dir, &mesh, critical_angle_deg);
        if new_score < best_score {
            best_dir = new_dir;
            best_score = new_score;
        }

        perturbation_deg *= 0.95;
        if i > iterations / 2 {
            perturbation_deg = perturbation_deg.max(0.5);
        }
    }

    vec![best_dir[0], best_dir[1], best_dir[2], best_score]
}
