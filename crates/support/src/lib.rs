pub mod types;
pub mod island;
pub mod volume;
pub mod placement;
pub mod raft;

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "wasm")]
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

/// WASM export: generate supports for a mesh given build direction and config.
///
/// Accepts flat f32 arrays (positions, normals, areas) and a direction vector,
/// plus a SupportConfig as JsValue (JSON-deserialized).
/// Returns a SupportResult as JsValue.
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn generate_supports(
    positions: &[f32],
    normals: &[f32],
    areas: &[f32],
    direction: &[f32],
    config: JsValue,
) -> JsValue {
    let config: types::SupportConfig = serde_wasm_bindgen::from_value(config)
        .unwrap_or_else(|e| wasm_bindgen::throw_str(&format!("Invalid config: {e}")));

    // 1. Detect islands
    let islands = island::detect_islands(positions, normals, direction, &config);

    // Compute mesh height range along build direction for raft plane
    let dir_array = [direction[0], direction[1], direction[2]];
    let mut min_height = f32::INFINITY;
    let tri_count = positions.len() / 9;
    for i in 0..tri_count {
        let base = i * 9;
        for j in 0..3 {
            let x = positions[base + j * 3];
            let y = positions[base + j * 3 + 1];
            let z = positions[base + j * 3 + 2];
            let height = -(x * dir_array[0] + y * dir_array[1] + z * dir_array[2]);
            min_height = min_height.min(height);
        }
    }
    let raft_height = min_height; // Raft plane at bottom of mesh

    // 2. For each island: classify volume, place contacts
    let mut all_contacts: Vec<(types::ContactPoint, f32)> = Vec::new();
    for island in &islands {
        // Convert centroid from grid cell coordinates to world coordinates
        let world_centroid = [
            island.centroid[0] * config.cell_size + island.grid_origin[0],
            island.centroid[1] * config.cell_size + island.grid_origin[1],
        ];
        let vol = volume::compute_volume_above(
            positions,
            normals,
            areas,
            &world_centroid,
            &dir_array,
        );
        let stype = volume::classify_support_type(vol, &config);
        let contacts = placement::place_contacts(
            island,
            positions,
            normals,
            &dir_array,
            &stype,
            &config,
            raft_height,
        );
        for c in contacts {
            all_contacts.push((c, vol));
        }
    }

    // 3. Generate raft
    let contact_points: Vec<types::ContactPoint> =
        all_contacts.iter().map(|(c, _)| c.clone()).collect();
    let raft = raft::generate_raft(&contact_points, &config);

    // 4. Build supports from contacts
    let supports: Vec<types::Support> = all_contacts
        .iter()
        .map(|(c, vol)| types::Support {
            contact: c.clone(),
            volume_above: *vol,
        })
        .collect();

    let total_volume: f32 = supports.iter().map(|s| s.volume_above).sum();

    let result = types::SupportResult {
        supports,
        raft,
        total_volume,
        island_count: islands.len() as u32,
    };

    serde_wasm_bindgen::to_value(&result).unwrap()
}

/// WASM export: return default SupportConfig as JsValue.
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn default_config() -> JsValue {
    serde_wasm_bindgen::to_value(&types::SupportConfig::default()).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_compiles() {
        // Basic smoke test that the crate compiles
        let _config = types::SupportConfig::default();
    }
}
