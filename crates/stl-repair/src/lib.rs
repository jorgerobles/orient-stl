/// Mesh repair pipeline — re-exports geometry-kernel flat operations as a coherent pipeline.

pub fn repair_positions(positions: &mut Vec<f32>, weld_epsilon: f32, max_hole_edges: u32) {
    geometry_kernel::flat::repair_mesh(positions);
    geometry_kernel::flat::normalize_winding(positions);
    if weld_epsilon > 0.0 {
        geometry_kernel::flat::weld_vertices(positions, weld_epsilon);
        geometry_kernel::flat::repair_mesh(positions);
    }
    if max_hole_edges > 0 {
        geometry_kernel::flat::fill_holes(positions, max_hole_edges);
    }
}

pub use geometry_kernel::flat::{
    DEFAULT_WELD_EPSILON, count_boundary_edges, fill_holes, normalize_winding, repair_mesh,
    weld_vertices,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repair_mesh_removes_duplicates() {
        let mut positions = vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.5, 1.0, 0.0,
        ];
        let removed = repair_mesh(&mut positions);
        assert_eq!(removed, 1);
        assert_eq!(positions.len() / 9, 1);
    }
}
