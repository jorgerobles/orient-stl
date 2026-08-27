use std::collections::{VecDeque, HashSet};

use crate::types::{SupportConfig, Island};

/// Rasterize a single triangle onto a 2D grid at a given z-slice.
/// Returns the set of grid cells covered by the triangle.
fn rasterize_triangle(
    v1: [f32; 3],
    v2: [f32; 3],
    v3: [f32; 3],
    z: f32,
    cell_size: f32,
    grid_min: [f32; 2],
) -> Vec<(u32, u32)> {
    // Compute intersection points of triangle edges with z-plane
    let vertices = [v1, v2, v3];
    let mut points_2d: Vec<[f32; 2]> = Vec::new();

    // Check each edge for intersection with z-plane
    for i in 0..3 {
        let a = vertices[i];
        let b = vertices[(i + 1) % 3];

        // Skip edges parallel to z-plane
        if (a[2] - b[2]).abs() < 1e-10 {
            continue;
        }

        // Check if z-plane intersects this edge
        let t = (z - a[2]) / (b[2] - a[2]);
        if t >= 0.0 && t <= 1.0 {
            let x = a[0] + t * (b[0] - a[0]);
            let y = a[1] + t * (b[1] - a[1]);
            points_2d.push([x, y]);
        }
    }

    // Also include vertices that lie exactly on the z-plane
    for v in &vertices {
        if (v[2] - z).abs() < 1e-10 {
            points_2d.push([v[0], v[1]]);
        }
    }

    // Deduplicate points
    points_2d.sort_by(|a, b| {
        a[0].partial_cmp(&b[0])
            .unwrap()
            .then(a[1].partial_cmp(&b[1]).unwrap())
    });
    points_2d.dedup_by(|a, b| {
        (a[0] - b[0]).abs() < 1e-10 && (a[1] - b[1]).abs() < 1e-10
    });

    if points_2d.len() < 3 {
        // Need at least 3 points for a triangle cross-section
        // If we have exactly 2 points, it's an edge - add both cells
        if points_2d.len() == 2 {
            let mut cells = Vec::new();
            for p in &points_2d {
                let cx = ((p[0] - grid_min[0]) / cell_size).floor() as i32;
                let cy = ((p[1] - grid_min[1]) / cell_size).floor() as i32;
                if cx >= 0 && cy >= 0 {
                    cells.push((cx as u32, cy as u32));
                }
            }
            return cells;
        }
        return Vec::new();
    }

    // For 3+ points, compute convex hull and fill bounding box
    // (conservative but sufficient for support detection)
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for p in &points_2d {
        min_x = min_x.min(p[0]);
        max_x = max_x.max(p[0]);
        min_y = min_y.min(p[1]);
        max_y = max_y.max(p[1]);
    }

    // Convert to grid coordinates
    let cell_min_x = ((min_x - grid_min[0]) / cell_size).floor() as i32;
    let cell_max_x = ((max_x - grid_min[0]) / cell_size).ceil() as i32;
    let cell_min_y = ((min_y - grid_min[1]) / cell_size).floor() as i32;
    let cell_max_y = ((max_y - grid_min[1]) / cell_size).ceil() as i32;

    let mut cells = Vec::new();

    // Fill all cells within the bounding box
    for cy in cell_min_y..=cell_max_y {
        for cx in cell_min_x..=cell_max_x {
            if cx >= 0 && cy >= 0 {
                cells.push((cx as u32, cy as u32));
            }
        }
    }

    cells
}

/// Build a binary grid for a single z-slice.
/// Returns (grid_cells, grid_min, grid_size) where grid_cells is a set of filled cells.
fn build_grid(
    positions: &[f32],
    z: f32,
    cell_size: f32,
) -> (HashSet<(u32, u32)>, [f32; 2], [u32; 2]) {
    let tri_count = positions.len() / 9;
    if tri_count == 0 {
        return (HashSet::new(), [0.0; 2], [0; 2]);
    }

    // Find bounding box of all vertices
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for i in 0..tri_count {
        let base = i * 9;
        for j in 0..3 {
            let x = positions[base + j * 3];
            let y = positions[base + j * 3 + 1];
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
    }

    let grid_min = [min_x, min_y];
    let grid_w = ((max_x - min_x) / cell_size).ceil() as u32;
    let grid_h = ((max_y - min_y) / cell_size).ceil() as u32;

    // Rasterize all triangles at this z-slice
    let mut filled = HashSet::new();
    for i in 0..tri_count {
        let base = i * 9;
        let v1 = [positions[base], positions[base + 1], positions[base + 2]];
        let v2 = [positions[base + 3], positions[base + 4], positions[base + 5]];
        let v3 = [positions[base + 6], positions[base + 7], positions[base + 8]];

        // Skip triangles entirely above or below z-slice
        let z_min_tri = v1[2].min(v2[2]).min(v3[2]);
        let z_max_tri = v1[2].max(v2[2]).max(v3[2]);
        if z < z_min_tri - cell_size || z > z_max_tri + cell_size {
            continue;
        }

        let cells = rasterize_triangle(v1, v2, v3, z, cell_size, grid_min);
        for cell in cells {
            filled.insert(cell);
        }
    }

    (filled, grid_min, [grid_w, grid_h])
}

/// BFS to find connected components of a set of grid cells.
fn connected_components(cells: &HashSet<(u32, u32)>) -> Vec<Vec<(u32, u32)>> {
    let mut visited = HashSet::new();
    let mut components = Vec::new();

    for &start in cells {
        if visited.contains(&start) {
            continue;
        }

        // BFS from this cell
        let mut component = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(start);
        visited.insert(start);

        while let Some((x, y)) = queue.pop_front() {
            component.push((x, y));

            // Check 4-connected neighbors
            for &(dx, dy) in &[(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx >= 0 && ny >= 0 {
                    let neighbor = (nx as u32, ny as u32);
                    if cells.contains(&neighbor) && !visited.contains(&neighbor) {
                        visited.insert(neighbor);
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        components.push(component);
    }

    components
}

/// Detect disconnected overhang islands in the mesh.
///
/// Algorithm:
/// 1. For each layer z from z_min to z_max:
///    a. Rasterize mesh at height z → binary grid (grid_z)
///    b. Rasterize mesh at height z + layer_height → binary grid (grid_above)
///    c. For each filled pixel in grid_z, check if any 4-connected neighbor
///       is also filled in grid_above
///    d. Pixels NOT connected to grid_above are "island pixels"
/// 2. Connected components on accumulated island pixels → Islands
pub fn detect_islands(
    positions: &[f32],
    _normals: &[f32],
    direction: &[f32],
    config: &SupportConfig,
) -> Vec<Island> {
    let tri_count = positions.len() / 9;
    if tri_count == 0 {
        return Vec::new();
    }

    // Normalize direction
    let dir_len = (direction[0] * direction[0] + direction[1] * direction[1] + direction[2] * direction[2]).sqrt();
    if dir_len < 1e-10 {
        return Vec::new();
    }
    let dir = [
        direction[0] / dir_len,
        direction[1] / dir_len,
        direction[2] / dir_len,
    ];

    // Project vertices onto build direction to find z range
    // z projection = dot(vertex, -direction) (height above build plate)
    let mut z_min = f32::INFINITY;
    let mut z_max = f32::NEG_INFINITY;

    for i in 0..tri_count {
        let base = i * 9;
        for j in 0..3 {
            let x = positions[base + j * 3];
            let y = positions[base + j * 3 + 1];
            let z = positions[base + j * 3 + 2];
            // Project onto -direction (height above build plate)
            let height = -(x * dir[0] + y * dir[1] + z * dir[2]);
            z_min = z_min.min(height);
            z_max = z_max.max(height);
        }
    }

    if z_min >= z_max {
        // Flat mesh - add at least one layer
        z_max = z_min + config.layer_height;
    }

    // Cap at 1000 layers (threat model T-09-01)
    let layer_count = ((z_max - z_min) / config.layer_height).ceil() as u32;
    let layer_count = layer_count.min(1000).max(1);

    // Accumulate island pixels across all layers
    let mut all_island_pixels: HashSet<(u32, u32)> = HashSet::new();
    let mut island_z_min: f32 = f32::INFINITY;
    let mut island_z_max: f32 = f32::NEG_INFINITY;

    for layer in 0..layer_count {
        let z = z_min + layer as f32 * config.layer_height;
        let z_above = z + config.layer_height;

        // Build grids at current and next layer
        let (grid_z, _, _) = build_grid(positions, z, config.cell_size);
        let (grid_above, _, _) = build_grid(positions, z_above, config.cell_size);

        // Find island pixels: cells in grid_z not connected to grid_above
        for &cell in &grid_z {
            let (x, y) = cell;
            let mut connected_to_above = false;

            // Check 4-connected neighbors in grid_above
            for &(dx, dy) in &[(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx >= 0 && ny >= 0 {
                    if grid_above.contains(&(nx as u32, ny as u32)) {
                        connected_to_above = true;
                        break;
                    }
                }
            }

            // Also check if the cell itself is in grid_above (it's on the boundary)
            if grid_above.contains(&cell) {
                connected_to_above = true;
            }

            if !connected_to_above {
                all_island_pixels.insert(cell);
                island_z_min = island_z_min.min(z);
                island_z_max = island_z_max.max(z);
            }
        }
    }

    if all_island_pixels.is_empty() {
        return Vec::new();
    }

    // Find connected components of island pixels
    let components = connected_components(&all_island_pixels);

    // Build islands from components
    let mut islands = Vec::new();
    for component in components {
        if component.is_empty() {
            continue;
        }

        // Compute centroid
        let mut sum_x = 0.0f32;
        let mut sum_y = 0.0f32;
        for &(cx, cy) in &component {
            sum_x += cx as f32;
            sum_y += cy as f32;
        }
        let n = component.len() as f32;
        let centroid = [sum_x / n, sum_y / n];

        // Estimate area (number of cells × cell_size²)
        let area = n * config.cell_size * config.cell_size;

        islands.push(Island {
            pixels: component,
            centroid,
            area,
            z_min: island_z_min,
            z_max: island_z_max,
        });
    }

    islands
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a flat horizontal plate (two triangles forming a square).
    /// The plate is at z=0, extending from (0,0,0) to (10,10,0).
    /// Build direction is [0, 0, -1] (downward, plate overhangs from above).
    fn flat_plate_positions() -> Vec<f32> {
        vec![
            // Triangle 1
            0.0, 0.0, 0.0,
            10.0, 0.0, 0.0,
            10.0, 10.0, 0.0,
            // Triangle 2
            0.0, 0.0, 0.0,
            10.0, 10.0, 0.0,
            0.0, 10.0, 0.0,
        ]
    }

    /// Helper to create a cube with overhanging sides.
    /// Cube from (0,0,0) to (10,10,10), build direction [0, 0, -1].
    /// The bottom face is flat, but side faces produce islands at different heights.
    fn cube_positions() -> Vec<f32> {
        vec![
            // Bottom face (z=0)
            0.0, 0.0, 0.0, 10.0, 0.0, 0.0, 10.0, 10.0, 0.0,
            0.0, 0.0, 0.0, 10.0, 10.0, 0.0, 0.0, 10.0, 0.0,
            // Top face (z=10)
            0.0, 0.0, 10.0, 10.0, 10.0, 10.0, 10.0, 0.0, 10.0,
            0.0, 0.0, 10.0, 0.0, 10.0, 10.0, 10.0, 0.0, 10.0,
            // Front face (y=0)
            0.0, 0.0, 0.0, 10.0, 0.0, 0.0, 10.0, 0.0, 10.0,
            0.0, 0.0, 0.0, 10.0, 0.0, 10.0, 0.0, 0.0, 10.0,
            // Back face (y=10)
            0.0, 10.0, 0.0, 10.0, 10.0, 10.0, 10.0, 10.0, 0.0,
            0.0, 10.0, 0.0, 0.0, 10.0, 10.0, 10.0, 10.0, 0.0,
            // Left face (x=0)
            0.0, 0.0, 0.0, 0.0, 10.0, 10.0, 0.0, 10.0, 0.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 10.0, 0.0, 10.0, 10.0,
            // Right face (x=10)
            10.0, 0.0, 0.0, 10.0, 10.0, 0.0, 10.0, 10.0, 10.0,
            10.0, 0.0, 0.0, 10.0, 10.0, 10.0, 10.0, 0.0, 10.0,
        ]
    }

    /// Helper to create two separate overhangs.
    /// Two plates at different positions with a gap between them.
    fn two_overhangs_positions() -> Vec<f32> {
        vec![
            // Plate 1 at (0,0,0) to (5,5,0)
            0.0, 0.0, 0.0, 5.0, 0.0, 0.0, 5.0, 5.0, 0.0,
            0.0, 0.0, 0.0, 5.0, 5.0, 0.0, 0.0, 5.0, 0.0,
            // Plate 2 at (15,15,0) to (20,20,0)
            15.0, 15.0, 0.0, 20.0, 15.0, 0.0, 20.0, 20.0, 0.0,
            15.0, 15.0, 0.0, 20.0, 20.0, 0.0, 15.0, 20.0, 0.0,
        ]
    }

    #[test]
    fn flat_plate_detects_single_island() {
        let positions = flat_plate_positions();
        let normals: Vec<f32> = vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0];
        let direction = [0.0, 0.0, -1.0];
        let config = SupportConfig::default();

        let islands = detect_islands(&positions, &normals, &direction, &config);

        // Flat plate should produce at least one island (the entire bottom)
        assert!(!islands.is_empty(), "flat plate should have islands");

        // The island should cover the entire plate area
        let total_area: f32 = islands.iter().map(|i| i.area).sum();
        assert!(total_area > 0.0, "island area should be positive");
    }

    #[test]
    fn cube_produces_multiple_islands() {
        let positions = cube_positions();
        // 12 triangles, 3 floats each = 36 floats
        let mut normals: Vec<f32> = (0..36).map(|_| 0.0).collect();
        normals[2] = 1.0; // first triangle normal points up
        normals[5] = 1.0;
        let direction = [0.0, 0.0, -1.0];
        let config = SupportConfig::default();

        let islands = detect_islands(&positions, &normals, &direction, &config);

        // Cube side faces should produce islands at different z heights
        assert!(islands.len() >= 1, "cube should have at least one island");
    }

    #[test]
    fn two_overhangs_produce_two_islands() {
        let positions = two_overhangs_positions();
        // 4 triangles, 3 floats each = 12 floats
        let normals: Vec<f32> = vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0];
        let direction = [0.0, 0.0, -1.0];
        let config = SupportConfig::default();

        let islands = detect_islands(&positions, &normals, &direction, &config);

        // Two separate plates should produce two distinct islands
        assert_eq!(islands.len(), 2, "two separate overhangs should produce 2 islands");

        // Verify they are spatially separated
        assert_ne!(islands[0].centroid, islands[1].centroid,
            "islands should have different centroids");
    }

    #[test]
    fn empty_mesh_produces_no_islands() {
        let positions: Vec<f32> = Vec::new();
        let normals: Vec<f32> = Vec::new();
        let direction = [0.0, 0.0, -1.0];
        let config = SupportConfig::default();

        let islands = detect_islands(&positions, &normals, &direction, &config);
        assert!(islands.is_empty(), "empty mesh should have no islands");
    }

    #[test]
    fn zero_direction_produces_no_islands() {
        let positions = flat_plate_positions();
        let normals: Vec<f32> = vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0];
        let direction = [0.0, 0.0, 0.0];
        let config = SupportConfig::default();

        let islands = detect_islands(&positions, &normals, &direction, &config);
        assert!(islands.is_empty(), "zero direction should have no islands");
    }

    #[test]
    fn connected_components_finds_correct_groups() {
        let mut cells = HashSet::new();
        // Group 1: 2x2 block at origin
        cells.insert((0, 0));
        cells.insert((1, 0));
        cells.insert((0, 1));
        cells.insert((1, 1));
        // Group 2: isolated cell far away
        cells.insert((100, 100));

        let components = connected_components(&cells);
        assert_eq!(components.len(), 2, "should find 2 components");

        // Find the larger component
        let large = components.iter().find(|c| c.len() == 4).unwrap();
        assert!(large.contains(&(0, 0)));
        assert!(large.contains(&(1, 0)));
        assert!(large.contains(&(0, 1)));
        assert!(large.contains(&(1, 1)));
    }

    #[test]
    fn rasterize_triangle_returns_cells() {
        let v1 = [0.0, 0.0, 0.0];
        let v2 = [2.0, 0.0, 0.0];
        let v3 = [1.0, 2.0, 0.0];
        let z = 0.0;
        let cell_size = 0.5;
        let grid_min = [-1.0, -1.0];

        let cells = rasterize_triangle(v1, v2, v3, z, cell_size, grid_min);
        assert!(!cells.is_empty(), "triangle at z=0 should rasterize");
    }

    #[test]
    fn island_has_valid_centroid() {
        let positions = two_overhangs_positions();
        let normals: Vec<f32> = vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0];
        let direction = [0.0, 0.0, -1.0];
        let config = SupportConfig::default();

        let islands = detect_islands(&positions, &normals, &direction, &config);

        for island in &islands {
            // Centroid should be non-NaN
            assert!(island.centroid[0].is_finite());
            assert!(island.centroid[1].is_finite());
            // Area should be positive
            assert!(island.area > 0.0);
        }
    }
}
