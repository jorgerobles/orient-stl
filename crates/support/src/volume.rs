use crate::types::{SupportConfig, SupportType};

/// Möller-Trumbore ray-triangle intersection test.
/// Returns Some(t) if the ray hits the triangle, where t is the distance along the ray.
fn ray_triangle_intersect(
    ray_origin: [f32; 3],
    ray_dir: [f32; 3],
    v0: [f32; 3],
    v1: [f32; 3],
    v2: [f32; 3],
) -> Option<f32> {
    let eps = 1e-8;
    let v0v1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
    let v0v2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

    // Begin calculating determinant - also used to calculate u parameter
    let pvec = [
        ray_dir[1] * v0v2[2] - ray_dir[2] * v0v2[1],
        ray_dir[2] * v0v2[0] - ray_dir[0] * v0v2[2],
        ray_dir[0] * v0v2[1] - ray_dir[1] * v0v2[0],
    ];

    // If determinant is near zero, ray lies in plane of triangle
    let det = v0v1[0] * pvec[0] + v0v1[1] * pvec[1] + v0v1[2] * pvec[2];
    if det.abs() < eps {
        return None;
    }

    let inv_det = 1.0 / det;

    // Calculate distance from v0 to ray origin
    let tvec = [
        ray_origin[0] - v0[0],
        ray_origin[1] - v0[1],
        ray_origin[2] - v0[2],
    ];

    // Calculate u parameter and test bounds
    let u = (tvec[0] * pvec[0] + tvec[1] * pvec[1] + tvec[2] * pvec[2]) * inv_det;
    if u < -eps || u > 1.0 + eps {
        return None;
    }

    // Prepare to test v parameter
    let qvec = [
        tvec[1] * v0v1[2] - tvec[2] * v0v1[1],
        tvec[2] * v0v1[0] - tvec[0] * v0v1[2],
        tvec[0] * v0v1[1] - tvec[1] * v0v1[0],
    ];

    // Calculate v parameter and test bounds
    let v = (ray_dir[0] * qvec[0] + ray_dir[1] * qvec[1] + ray_dir[2] * qvec[2]) * inv_det;
    if v < -eps || u + v > 1.0 + eps {
        return None;
    }

    // Calculate t, ray intersects triangle
    let t = (v0v2[0] * qvec[0] + v0v2[1] * qvec[1] + v0v2[2] * qvec[2]) * inv_det;

    if t > eps {
        Some(t)
    } else {
        None
    }
}

/// Compute the volume of material above a 2D point by casting a ray upward.
///
/// The ray is cast from the point along -direction (upward relative to build plate).
/// For each intersected triangle, we accumulate area × distance to estimate volume.
pub fn compute_volume_above(
    positions: &[f32],
    _normals: &[f32],
    areas: &[f32],
    point: &[f32; 2],
    direction: &[f32; 3],
) -> f32 {
    let tri_count = positions.len() / 9;
    if tri_count == 0 {
        return 0.0;
    }

    // Normalize direction
    let dir_len = (direction[0] * direction[0] + direction[1] * direction[1] + direction[2] * direction[2]).sqrt();
    if dir_len < 1e-10 {
        return 0.0;
    }
    let dir = [
        direction[0] / dir_len,
        direction[1] / dir_len,
        direction[2] / dir_len,
    ];

    // Find the height range of the mesh along the build direction
    let mut min_height = f32::INFINITY;
    let mut max_height = f32::NEG_INFINITY;
    for i in 0..tri_count {
        let base = i * 9;
        for j in 0..3 {
            let x = positions[base + j * 3];
            let y = positions[base + j * 3 + 1];
            let z = positions[base + j * 3 + 2];
            let height = -(x * dir[0] + y * dir[1] + z * dir[2]);
            min_height = min_height.min(height);
            max_height = max_height.max(height);
        }
    }

    // Cast ray from well below the mesh upward (opposite to build direction)
    // This finds all geometry above the 2D point
    let ray_dir = [-dir[0], -dir[1], -dir[2]];
    let ray_origin = [
        point[0],
        point[1],
        min_height - 1.0,
    ];

    // Find all intersections
    let mut intersections: Vec<f32> = Vec::new();

    for i in 0..tri_count {
        let base = i * 9;
        let v0 = [positions[base], positions[base + 1], positions[base + 2]];
        let v1 = [positions[base + 3], positions[base + 4], positions[base + 5]];
        let v2 = [positions[base + 6], positions[base + 7], positions[base + 8]];

        if let Some(t) = ray_triangle_intersect(ray_origin, ray_dir, v0, v1, v2) {
            intersections.push(t);
        }
    }

    if intersections.is_empty() {
        return 0.0;
    }

    // Sort intersections by distance
    intersections.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // Accumulate volume: sum of area × distance between consecutive intersections
    let mut volume = 0.0f32;
    let mut prev_t = 0.0f32;

    for (i, &t) in intersections.iter().enumerate() {
        if i > 0 {
            let distance = t - prev_t;
            // Use the average area of intersected triangles as a proxy
            // In a more accurate implementation, we'd compute the cross-sectional area
            let avg_area: f32 = areas.iter().sum::<f32>() / areas.len() as f32;
            volume += avg_area * distance;
        }
        prev_t = t;
    }

    volume
}

/// Classify support type based on volume above.
pub fn classify_support_type(volume: f32, config: &SupportConfig) -> SupportType {
    if volume < config.light_threshold {
        SupportType::Light
    } else if volume < config.medium_threshold {
        SupportType::Medium
    } else {
        SupportType::Heavy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a flat horizontal plate.
    fn flat_plate() -> (Vec<f32>, Vec<f32>, Vec<f32>) {
        let positions = vec![
            0.0, 0.0, 0.0,
            10.0, 0.0, 0.0,
            10.0, 10.0, 0.0,
            0.0, 0.0, 0.0,
            10.0, 10.0, 0.0,
            0.0, 10.0, 0.0,
        ];
        let normals = vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0];
        let areas = vec![50.0, 50.0]; // each triangle is 10x10/2 = 50
        (positions, normals, areas)
    }

    #[test]
    fn ray_triangle_intersect_basic() {
        // Triangle at z=0
        let v0 = [0.0, 0.0, 0.0];
        let v1 = [10.0, 0.0, 0.0];
        let v2 = [5.0, 10.0, 0.0];

        // Ray from above pointing down
        let origin = [5.0, 5.0, 10.0];
        let dir = [0.0, 0.0, -1.0];

        let result = ray_triangle_intersect(origin, dir, v0, v1, v2);
        assert!(result.is_some(), "ray should hit triangle");
        let t = result.unwrap();
        assert!((t - 10.0).abs() < 1e-6, "intersection at t=10");
    }

    #[test]
    fn ray_triangle_intersect_miss() {
        // Triangle at z=0
        let v0 = [0.0, 0.0, 0.0];
        let v1 = [10.0, 0.0, 0.0];
        let v2 = [5.0, 10.0, 0.0];

        // Ray from far away, pointing away from triangle
        let origin = [5.0, 5.0, -10.0];
        let dir = [0.0, 0.0, -1.0];

        let result = ray_triangle_intersect(origin, dir, v0, v1, v2);
        assert!(result.is_none(), "ray should miss triangle");
    }

    #[test]
    fn volume_above_flat_plate_is_positive() {
        // Create a box: bottom face at z=0, top face at z=10
        // Cast ray from center upward — should intersect both faces
        // Volume = area × height between intersections
        let positions = vec![
            // Bottom face (z=0)
            0.0, 0.0, 0.0,
            10.0, 0.0, 0.0,
            10.0, 10.0, 0.0,
            0.0, 0.0, 0.0,
            10.0, 10.0, 0.0,
            0.0, 10.0, 0.0,
            // Top face (z=10)
            0.0, 0.0, 10.0,
            10.0, 10.0, 10.0,
            10.0, 0.0, 10.0,
            0.0, 0.0, 10.0,
            0.0, 10.0, 10.0,
            10.0, 10.0, 10.0,
        ];
        let normals = vec![
            0.0, 0.0, 1.0, 0.0, 0.0, 1.0,
            0.0, 0.0, -1.0, 0.0, 0.0, -1.0,
        ];
        let areas = vec![50.0, 50.0, 50.0, 50.0];
        let point = [5.0, 5.0]; // center
        let direction = [0.0, 0.0, -1.0];

        let volume = compute_volume_above(&positions, &normals, &areas, &point, &direction);
        assert!(volume > 0.0, "volume above should be positive for enclosed geometry, got {}", volume);
    }

    #[test]
    fn classify_support_type_thresholds() {
        let config = SupportConfig::default();

        assert_eq!(classify_support_type(0.0, &config), SupportType::Light);
        assert_eq!(classify_support_type(49.9, &config), SupportType::Light);
        assert_eq!(classify_support_type(50.0, &config), SupportType::Medium);
        assert_eq!(classify_support_type(499.9, &config), SupportType::Medium);
        assert_eq!(classify_support_type(500.0, &config), SupportType::Heavy);
        assert_eq!(classify_support_type(1000.0, &config), SupportType::Heavy);
    }

    #[test]
    fn empty_mesh_volume_is_zero() {
        let positions: Vec<f32> = Vec::new();
        let normals: Vec<f32> = Vec::new();
        let areas: Vec<f32> = Vec::new();
        let point = [5.0, 5.0];
        let direction = [0.0, 0.0, -1.0];

        let volume = compute_volume_above(&positions, &normals, &areas, &point, &direction);
        assert_eq!(volume, 0.0);
    }

    #[test]
    fn zero_direction_volume_is_zero() {
        let (positions, normals, areas) = flat_plate();
        let point = [5.0, 5.0];
        let direction = [0.0, 0.0, 0.0];

        let volume = compute_volume_above(&positions, &normals, &areas, &point, &direction);
        assert_eq!(volume, 0.0);
    }
}
