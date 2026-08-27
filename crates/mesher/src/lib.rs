/// Mesh precomputation — normals, areas, vertices.

pub struct MeshOutput {
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
    pub areas: Vec<f32>,
}

pub fn precompute_mesh_data(positions: &[f32]) -> Result<MeshOutput, String> {
    let triangle_count = positions.len() / 9;
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(triangle_count);
    let mut areas: Vec<f32> = Vec::with_capacity(triangle_count);
    let mut clean_vertices: Vec<[f32; 3]> = Vec::with_capacity(triangle_count * 3);

    for i in 0..triangle_count {
        let base = i * 9;
        let v1 = [positions[base], positions[base + 1], positions[base + 2]];
        let v2 = [positions[base + 3], positions[base + 4], positions[base + 5]];
        let v3 = [positions[base + 6], positions[base + 7], positions[base + 8]];

        let e1 = [v2[0] - v1[0], v2[1] - v1[1], v2[2] - v1[2]];
        let e2 = [v3[0] - v1[0], v3[1] - v1[1], v3[2] - v1[2]];

        let cx = e1[1] * e2[2] - e1[2] * e2[1];
        let cy = e1[2] * e2[0] - e1[0] * e2[2];
        let cz = e1[0] * e2[1] - e1[1] * e2[0];

        let area_sq = cx * cx + cy * cy + cz * cz;
        if area_sq <= f32::EPSILON {
            continue;
        }

        let area = 0.5 * area_sq.sqrt();
        let inv_len = 1.0 / (2.0 * area);
        normals.push([cx * inv_len, cy * inv_len, cz * inv_len]);
        areas.push(area);
        clean_vertices.push(v1);
        clean_vertices.push(v2);
        clean_vertices.push(v3);
    }

    if normals.is_empty() {
        return Err("All triangles are degenerate".into());
    }

    let normals_flat: Vec<f32> = normals.iter().flat_map(|n| n.iter()).copied().collect();
    let clean: Vec<f32> = clean_vertices.iter().flat_map(|v| v.iter()).copied().collect();

    Ok(MeshOutput {
        positions: clean,
        normals: normals_flat,
        areas,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn right_triangle_xy_plane() {
        let positions: Vec<f32> = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let out = precompute_mesh_data(&positions).unwrap();
        assert_eq!(out.areas.len(), 1);
        assert!((out.areas[0] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn degenerate_triangle_filtered() {
        let positions: Vec<f32> = vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        let result = precompute_mesh_data(&positions);
        assert!(result.is_err());
    }
}
