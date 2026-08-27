use crate::types::{SupportConfig, Island, ContactPoint, SupportType};

/// Simple pseudo-random number generator for Poisson-disk sampling.
struct Rng {
    state: u32,
}

impl Rng {
    fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    fn next_f32(&mut self) -> f32 {
        // xorshift32
        self.state ^= self.state << 13;
        self.state ^= self.state >> 17;
        self.state ^= self.state << 5;
        self.state as f32 / u32::MAX as f32
    }
}

/// Place contact points on an island using variable-density Poisson-disk sampling.
///
/// Spacing is based on support type:
/// - Light: 2.5-6mm
/// - Medium: 2-5mm
/// - Heavy: 1.5-3.5mm
///
/// Edge/corner seeding adds extra points at island boundaries.
pub fn place_contacts(
    island: &Island,
    positions: &[f32],
    normals: &[f32],
    direction: &[f32; 3],
    support_type: &SupportType,
    config: &SupportConfig,
    raft_height: f32,
) -> Vec<ContactPoint> {
    if island.pixels.is_empty() {
        return Vec::new();
    }

    // Get spacing range based on support type
    let (min_spacing, _max_spacing) = match support_type {
        SupportType::Light => config.light_spacing,
        SupportType::Medium => config.medium_spacing,
        SupportType::Heavy => config.heavy_spacing,
    };

    // Get tip diameter and penetration
    let (tip_diameter, penetration) = match support_type {
        SupportType::Light => (config.light_tip_diameter, config.light_penetration),
        SupportType::Medium => (config.medium_tip_diameter, config.medium_penetration),
        SupportType::Heavy => (config.heavy_tip_diameter, config.heavy_penetration),
    };

    let mut rng = Rng::new(island.centroid[0].to_bits() ^ island.centroid[1].to_bits());
    let mut contacts = Vec::new();
    let mut placed: Vec<[f32; 2]> = Vec::new();

    // Poisson-disk sampling with edge seeding
    let max_attempts = island.pixels.len() * 4;
    let mut attempts = 0;

    // First, seed edge points
    let edge_pixels = find_edge_pixels(island);
    for &pixel in &edge_pixels {
        let world_x = pixel.0 as f32 * config.cell_size + island.grid_origin[0];
        let world_y = pixel.1 as f32 * config.cell_size + island.grid_origin[1];

        // Check minimum distance from existing points
        if !too_close(&[world_x, world_y], &placed, min_spacing) {
            if let Some(contact) = create_contact_point(
                [world_x, world_y],
                island,
                positions,
                normals,
                direction,
                support_type,
                tip_diameter,
                penetration,
                config,
                raft_height,
            ) {
                contacts.push(contact);
                placed.push([world_x, world_y]);
            }
        }
    }

    // Then fill interior with Poisson-disk sampling
    while attempts < max_attempts {
        attempts += 1;

        // Random point within island bounding box
        let pixel_idx = (rng.next_f32() * island.pixels.len() as f32) as usize;
        let pixel = island.pixels[pixel_idx.min(island.pixels.len() - 1)];

        let world_x = pixel.0 as f32 * config.cell_size + island.grid_origin[0] + rng.next_f32() * config.cell_size;
        let world_y = pixel.1 as f32 * config.cell_size + island.grid_origin[1] + rng.next_f32() * config.cell_size;

        // Check if point is within island bounds (approximate)
        if !within_island_bounds([world_x, world_y], island, config) {
            continue;
        }

        // Check minimum distance from existing points
        if too_close(&[world_x, world_y], &placed, min_spacing) {
            continue;
        }

        if let Some(contact) = create_contact_point(
            [world_x, world_y],
            island,
            positions,
            normals,
            direction,
            support_type,
            tip_diameter,
            penetration,
            config,
            raft_height,
        ) {
            contacts.push(contact);
            placed.push([world_x, world_y]);
        }
    }

    contacts
}

/// Find edge pixels of an island (pixels with fewer than 4 neighbors).
fn find_edge_pixels(island: &Island) -> Vec<(u32, u32)> {
    let pixel_set: std::collections::HashSet<(u32, u32)> =
        island.pixels.iter().cloned().collect();

    let mut edge_pixels = Vec::new();
    for &(x, y) in &island.pixels {
        let mut neighbor_count = 0;
        for &(dx, dy) in &[(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx >= 0 && ny >= 0 && pixel_set.contains(&(nx as u32, ny as u32)) {
                neighbor_count += 1;
            }
        }
        if neighbor_count < 4 {
            edge_pixels.push((x, y));
        }
    }

    edge_pixels
}

/// Check if a point is too close to any existing point.
fn too_close(point: &[f32; 2], existing: &[[f32; 2]], min_distance: f32) -> bool {
    for &p in existing {
        let dx = point[0] - p[0];
        let dy = point[1] - p[1];
        let dist = (dx * dx + dy * dy).sqrt();
        if dist < min_distance {
            return true;
        }
    }
    false
}

/// Check if a point is within island bounds (approximate).
fn within_island_bounds(point: [f32; 2], island: &Island, config: &SupportConfig) -> bool {
    // Simple bounding box check
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for &(x, y) in &island.pixels {
        let wx = x as f32 * config.cell_size + island.grid_origin[0];
        let wy = y as f32 * config.cell_size + island.grid_origin[1];
        min_x = min_x.min(wx);
        max_x = max_x.max(wx);
        min_y = min_y.min(wy);
        max_y = max_y.max(wy);
    }

    point[0] >= min_x && point[0] <= max_x + config.cell_size &&
    point[1] >= min_y && point[1] <= max_y + config.cell_size
}

/// Create a contact point by projecting a 2D point back to 3D mesh surface.
fn create_contact_point(
    point_2d: [f32; 2],
    island: &Island,
    positions: &[f32],
    normals: &[f32],
    direction: &[f32; 3],
    support_type: &SupportType,
    tip_diameter: f32,
    penetration: f32,
    config: &SupportConfig,
    raft_height: f32,
) -> Option<ContactPoint> {
    let tri_count = positions.len() / 9;
    if tri_count == 0 {
        return None;
    }

    // Find nearest triangle to the 2D point
    let mut best_dist = f32::INFINITY;
    let mut best_point = [0.0f32; 3];
    let mut _best_normal = [0.0f32; 3];

    // Normalize direction
    let dir_len = (direction[0] * direction[0] + direction[1] * direction[1] + direction[2] * direction[2]).sqrt();
    if dir_len < 1e-10 {
        return None;
    }
    let dir = [
        direction[0] / dir_len,
        direction[1] / dir_len,
        direction[2] / dir_len,
    ];

    // Cast ray downward (opposite to build direction) from above to find mesh surface
    let ray_origin = [point_2d[0], point_2d[1], island.z_max + 1.0];
    let ray_dir = [-dir[0], -dir[1], -dir[2]]; // opposite to build direction (toward build plate)

    for i in 0..tri_count {
        let base = i * 9;
        let v0 = [positions[base], positions[base + 1], positions[base + 2]];
        let v1 = [positions[base + 3], positions[base + 4], positions[base + 5]];
        let v2 = [positions[base + 6], positions[base + 7], positions[base + 8]];

        // Simple ray-triangle intersection
        if let Some(t) = ray_triangle_intersect(ray_origin, ray_dir, v0, v1, v2) {
            let hit_point = [
                ray_origin[0] + t * ray_dir[0],
                ray_origin[1] + t * ray_dir[1],
                ray_origin[2] + t * ray_dir[2],
            ];

            let dist = (hit_point[0] - point_2d[0]).powi(2)
                + (hit_point[1] - point_2d[1]).powi(2);

            if dist < best_dist {
                best_dist = dist;
                best_point = hit_point;

                // Get normal from normals array
                if i * 3 + 2 < normals.len() {
                    _best_normal = [normals[i * 3], normals[i * 3 + 1], normals[i * 3 + 2]];
                }
            }
        }
    }

    if best_dist == f32::INFINITY {
        return None;
    }

    // Normalize direction
    let dir_len = (direction[0] * direction[0] + direction[1] * direction[1] + direction[2] * direction[2]).sqrt();
    let dir = if dir_len > 1e-10 {
        [direction[0] / dir_len, direction[1] / dir_len, direction[2] / dir_len]
    } else {
        [0.0, 1.0, 0.0]
    };

    // Project contact point down along build direction to raft plane
    // contact_height = dot(contact, -dir), raft_height is the bottom
    let contact_height = -(best_point[0] * dir[0] + best_point[1] * dir[1] + best_point[2] * dir[2]);
    let distance_to_raft = contact_height - raft_height;
    let base = [
        best_point[0] + dir[0] * distance_to_raft,
        best_point[1] + dir[1] * distance_to_raft,
        best_point[2] + dir[2] * distance_to_raft,
    ];

    Some(ContactPoint {
        position: best_point,
        base,
        support_type: support_type.clone(),
        tip_diameter,
        penetration,
    })
}

/// Simple ray-triangle intersection (Möller-Trumbore).
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

    let pvec = [
        ray_dir[1] * v0v2[2] - ray_dir[2] * v0v2[1],
        ray_dir[2] * v0v2[0] - ray_dir[0] * v0v2[2],
        ray_dir[0] * v0v2[1] - ray_dir[1] * v0v2[0],
    ];

    let det = v0v1[0] * pvec[0] + v0v1[1] * pvec[1] + v0v1[2] * pvec[2];
    if det.abs() < eps {
        return None;
    }

    let inv_det = 1.0 / det;

    let tvec = [
        ray_origin[0] - v0[0],
        ray_origin[1] - v0[1],
        ray_origin[2] - v0[2],
    ];

    let u = (tvec[0] * pvec[0] + tvec[1] * pvec[1] + tvec[2] * pvec[2]) * inv_det;
    if u < -eps || u > 1.0 + eps {
        return None;
    }

    let qvec = [
        tvec[1] * v0v1[2] - tvec[2] * v0v1[1],
        tvec[2] * v0v1[0] - tvec[0] * v0v1[2],
        tvec[0] * v0v1[1] - tvec[1] * v0v1[0],
    ];

    let v = (ray_dir[0] * qvec[0] + ray_dir[1] * qvec[1] + ray_dir[2] * qvec[2]) * inv_det;
    if v < -eps || u + v > 1.0 + eps {
        return None;
    }

    let t = (v0v2[0] * qvec[0] + v0v2[1] * qvec[1] + v0v2[2] * qvec[2]) * inv_det;

    if t > eps {
        Some(t)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_island() -> Island {
        Island {
            pixels: vec![(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)],
            grid_origin: [0.0, 0.0],
            centroid: [1.0, 0.5],
            area: 6.0 * 0.5 * 0.5, // 6 cells × 0.5mm × 0.5mm
            z_min: 0.0,
            z_max: 0.0,
        }
    }

    #[test]
    fn place_contacts_returns_contacts() {
        // Use a larger island so Medium spacing (2.0mm min) can place contacts
        let island = Island {
            pixels: (0..20).flat_map(|x| (0..10).map(move |y| (x, y))).collect(),
            grid_origin: [0.0, 0.0],
            centroid: [10.0, 5.0],
            area: 200.0 * 0.5 * 0.5, // 200 cells × 0.5mm × 0.5mm
            z_min: 0.0,
            z_max: 0.0,
        };
        let mut positions = Vec::new();
        // Create a large triangle covering the island area
        positions.extend_from_slice(&[0.0, 0.0, 0.0]);
        positions.extend_from_slice(&[20.0, 0.0, 0.0]);
        positions.extend_from_slice(&[10.0, 10.0, 0.0]);
        let normals = vec![0.0, 0.0, 1.0];
        let direction = [0.0, 0.0, -1.0];
        let config = SupportConfig::default();

        let contacts = place_contacts(
            &island,
            &positions,
            &normals,
            &direction,
            &SupportType::Medium,
            &config,
            -1.0, // raft_height: bottom of mesh
        );

        assert!(!contacts.is_empty(), "should place at least one contact");
    }

    #[test]
    fn contacts_have_valid_positions() {
        let island = test_island();
        let positions = vec![
            0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 3.0, 2.0, 0.0,
            0.0, 0.0, 0.0, 3.0, 2.0, 0.0, 0.0, 2.0, 0.0,
        ];
        let normals = vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0];
        let direction = [0.0, 0.0, -1.0];
        let config = SupportConfig::default();

        let contacts = place_contacts(
            &island,
            &positions,
            &normals,
            &direction,
            &SupportType::Light,
            &config,
            0.0, // raft_height
        );

        for contact in &contacts {
            assert!(contact.position[0].is_finite());
            assert!(contact.position[1].is_finite());
            assert!(contact.position[2].is_finite());
            assert!(contact.base[0].is_finite());
            assert!(contact.base[1].is_finite());
            assert!(contact.base[2].is_finite());
            assert!(contact.tip_diameter > 0.0);
            assert!(contact.penetration > 0.0);
        }
    }

    #[test]
    fn empty_island_returns_no_contacts() {
        let island = Island {
            pixels: vec![],
            grid_origin: [0.0, 0.0],
            centroid: [0.0, 0.0],
            area: 0.0,
            z_min: 0.0,
            z_max: 0.0,
        };
        let config = SupportConfig::default();

        let contacts = place_contacts(
            &island,
            &[],
            &[],
            &[0.0, 0.0, -1.0],
            &SupportType::Light,
            &config,
            0.0, // raft_height
        );

        assert!(contacts.is_empty());
    }

    #[test]
    fn find_edge_pixels_identifies_boundary() {
        let island = Island {
            pixels: vec![(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)],
            grid_origin: [0.0, 0.0],
            centroid: [1.0, 0.5],
            area: 1.5,
            z_min: 0.0,
            z_max: 0.0,
        };

        let edge = find_edge_pixels(&island);

        // All pixels in a 3x2 grid are edge pixels (no interior pixels)
        assert_eq!(edge.len(), 6);
    }
}
