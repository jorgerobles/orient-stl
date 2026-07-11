pub(crate) struct MeshData {
    pub normals: Vec<[f32; 3]>,
    pub areas: Vec<f32>,
    pub vertices: Vec<[f32; 3]>,
    pub triangle_count: usize,
}

pub(crate) fn precompute_mesh(positions: &[f32]) -> MeshData {
    let triangle_count = positions.len() / 9;
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(triangle_count);
    let mut areas: Vec<f32> = Vec::with_capacity(triangle_count);
    let mut vertices: Vec<[f32; 3]> = Vec::with_capacity(triangle_count * 3);

    for i in 0..triangle_count {
        let base = i * 9;
        let v1 = [
            positions[base],
            positions[base + 1],
            positions[base + 2],
        ];
        let v2 = [
            positions[base + 3],
            positions[base + 4],
            positions[base + 5],
        ];
        let v3 = [
            positions[base + 6],
            positions[base + 7],
            positions[base + 8],
        ];

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
        let nx = cx * inv_len;
        let ny = cy * inv_len;
        let nz = cz * inv_len;

        normals.push([nx, ny, nz]);
        areas.push(area);
        vertices.push(v1);
        vertices.push(v2);
        vertices.push(v3);
    }

    let triangle_count = normals.len();
    MeshData {
        normals,
        areas,
        vertices,
        triangle_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn right_triangle_xy_plane() {
        // Right triangle in XY plane → normal should be (0, 0, 1)
        let positions: Vec<f32> = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let mesh = precompute_mesh(&positions);
        assert_eq!(mesh.triangle_count, 1);
        assert!((mesh.normals[0][2] - 1.0).abs() < 1e-6);
        assert!((mesh.areas[0] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn degenerate_triangle_filtered() {
        // Two vertices at same position → zero area → filtered
        let positions: Vec<f32> = vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        let mesh = precompute_mesh(&positions);
        assert_eq!(mesh.triangle_count, 0);
    }

    #[test]
    fn normals_are_unit_length() {
        let positions: Vec<f32> = vec![
            0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 2.0, 0.0,
            1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 1.0,
        ];
        let mesh = precompute_mesh(&positions);
        for n in &mesh.normals {
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            assert!((len - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn mixed_valid_and_degenerate() {
        let positions: Vec<f32> = vec![
            // degenerate
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0,
            // valid
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
        ];
        let mesh = precompute_mesh(&positions);
        assert_eq!(mesh.triangle_count, 1);
    }
}
