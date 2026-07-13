use crate::mesh::MeshData;
use crate::hull::ConvexHull;

pub struct StabilityResult {
    pub stable: bool,
    pub margin: f32,
    pub contact_area: f32,
}

pub fn check_stability(
    direction: &[f32; 3],
    mesh: &MeshData,
    hull: &ConvexHull,
) -> StabilityResult {
    let dn_ln = (direction[0] * direction[0] + direction[1] * direction[1] + direction[2] * direction[2]).sqrt();
    if dn_ln < 1e-8 {
        return StabilityResult { stable: true, margin: 1.0, contact_area: 0.0 };
    }
    let dn = [direction[0] / dn_ln, direction[1] / dn_ln, direction[2] / dn_ln];

    let up = [-dn[0], -dn[1], -dn[2]];

    let (up_x, up_y) = find_perpendicular(up);

    let mut min_dot = f32::MAX;
    let mut max_dot = f32::MIN;
    for v in &mesh.vertices {
        let d = v[0] * dn[0] + v[1] * dn[1] + v[2] * dn[2];
        if d < min_dot { min_dot = d; }
        if d > max_dot { max_dot = d; }
    }

    let eps = 1e-4;
    let z_min = min_dot;
    let mut footprint_pts: Vec<[f32; 2]> = Vec::new();
    for v in &mesh.vertices {
        let d = v[0] * dn[0] + v[1] * dn[1] + v[2] * dn[2];
        if (d - z_min).abs() < eps {
            let x = v[0] * up_x[0] + v[1] * up_x[1] + v[2] * up_x[2];
            let y = v[0] * up_y[0] + v[1] * up_y[1] + v[2] * up_y[2];
            footprint_pts.push([x, y]);
        }
    }

    if footprint_pts.len() < 3 {
        return StabilityResult { stable: true, margin: 1.0, contact_area: 0.0 };
    }

    let hull_2d = convex_hull_2d(&footprint_pts);
    let contact_area = polygon_area(&hull_2d);

    let mut com = [0.0f32; 3];
    for v in &mesh.vertices {
        com[0] += v[0];
        com[1] += v[1];
        com[2] += v[2];
    }
    let n = mesh.vertices.len() as f32;
    com = [com[0] / n, com[1] / n, com[2] / n];

    let com_x = com[0] * up_x[0] + com[1] * up_x[1] + com[2] * up_x[2];
    let com_y = com[0] * up_y[0] + com[1] * up_y[1] + com[2] * up_y[2];

    let inside = point_in_convex_polygon(&[com_x, com_y], &hull_2d);
    if !inside {
        return StabilityResult { stable: false, margin: 0.0, contact_area };
    }

    let margin = min_edge_distance(&[com_x, com_y], &hull_2d);
    let norm_margin = if contact_area > 1e-8 {
        margin / contact_area.sqrt()
    } else {
        1.0
    };

    let _ = hull;
    StabilityResult {
        stable: true,
        margin: norm_margin,
        contact_area,
    }
}

fn find_perpendicular(v: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let x_axis = if v[0].abs() < 0.9 { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] };
    let mut up_x = [
        v[1] * x_axis[2] - v[2] * x_axis[1],
        v[2] * x_axis[0] - v[0] * x_axis[2],
        v[0] * x_axis[1] - v[1] * x_axis[0],
    ];
    let len = (up_x[0] * up_x[0] + up_x[1] * up_x[1] + up_x[2] * up_x[2]).sqrt();
    if len > 1e-8 {
        up_x = [up_x[0] / len, up_x[1] / len, up_x[2] / len];
    }

    let up_y = [
        v[1] * up_x[2] - v[2] * up_x[1],
        v[2] * up_x[0] - v[0] * up_x[2],
        v[0] * up_x[1] - v[1] * up_x[0],
    ];
    (up_x, up_y)
}

fn convex_hull_2d(points: &[[f32; 2]]) -> Vec<[f32; 2]> {
    if points.len() < 3 {
        return points.to_vec();
    }

    let mut pts: Vec<(f32, f32)> = points.iter().map(|&[x, y]| (x, y)).collect();
    pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.1.partial_cmp(&b.1).unwrap()));

    let cross = |o: (f32, f32), a: (f32, f32), b: (f32, f32)| -> f32 {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    };

    let mut lower = Vec::new();
    for &p in &pts {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], p) <= 0.0 {
            lower.pop();
        }
        lower.push(p);
    }

    let mut upper = Vec::new();
    for &p in pts.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], p) <= 0.0 {
            upper.pop();
        }
        upper.push(p);
    }

    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower.into_iter().map(|(x, y)| [x, y]).collect()
}

fn polygon_area(poly: &[[f32; 2]]) -> f32 {
    let n = poly.len();
    if n < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        area += poly[i][0] * poly[j][1];
        area -= poly[j][0] * poly[i][1];
    }
    area.abs() * 0.5
}

fn point_in_convex_polygon(point: &[f32; 2], poly: &[[f32; 2]]) -> bool {
    let n = poly.len();
    if n < 3 {
        return true;
    }
    let mut sign = 0.0f32;
    for i in 0..n {
        let j = (i + 1) % n;
        let cross = (poly[j][0] - poly[i][0]) * (point[1] - poly[i][1])
            - (poly[j][1] - poly[i][1]) * (point[0] - poly[i][0]);
        if cross.abs() < 1e-6 {
            continue;
        }
        let s = cross.signum();
        if sign == 0.0 {
            sign = s;
        } else if (s - sign).abs() > 0.1 {
            return false;
        }
    }
    true
}

fn min_edge_distance(point: &[f32; 2], poly: &[[f32; 2]]) -> f32 {
    let n = poly.len();
    if n < 3 {
        return f32::MAX;
    }
    let mut min_dist = f32::MAX;
    for i in 0..n {
        let j = (i + 1) % n;
        let dx = poly[j][0] - poly[i][0];
        let dy = poly[j][1] - poly[i][1];
        let len_sq = dx * dx + dy * dy;
        if len_sq < 1e-12 {
            continue;
        }
        let t = ((point[0] - poly[i][0]) * dx + (point[1] - poly[i][1]) * dy) / len_sq;
        let t = t.clamp(0.0, 1.0);
        let px = poly[i][0] + t * dx;
        let py = poly[i][1] + t * dy;
        let d = ((point[0] - px) * (point[0] - px) + (point[1] - py) * (point[1] - py)).sqrt();
        if d < min_dist {
            min_dist = d;
        }
    }
    min_dist
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::precompute_mesh;
    use crate::hull::compute_hull;

    fn cube_mesh() -> MeshData {
        let mut positions = Vec::new();
        let verts = [
            [-1.0, -1.0, -1.0], [1.0, -1.0, -1.0], [1.0, -1.0, 1.0], [-1.0, -1.0, 1.0],
            [-1.0, 1.0, -1.0], [1.0, 1.0, -1.0], [1.0, 1.0, 1.0], [-1.0, 1.0, 1.0],
        ];
        let faces: [[usize; 3]; 12] = [
            [0, 1, 2], [0, 2, 3],
            [4, 6, 5], [4, 7, 6],
            [0, 4, 5], [0, 5, 1],
            [1, 5, 6], [1, 6, 2],
            [2, 6, 7], [2, 7, 3],
            [3, 7, 4], [3, 4, 0],
        ];
        for &[a, b, c] in &faces {
            for &idx in &[a, b, c] {
                positions.push(verts[idx][0]);
                positions.push(verts[idx][1]);
                positions.push(verts[idx][2]);
            }
        }
        precompute_mesh(&positions)
    }

    fn cube_hull() -> ConvexHull {
        let verts: Vec<[f32; 3]> = [
            [-1.0, -1.0, -1.0], [1.0, -1.0, -1.0], [1.0, -1.0, 1.0], [-1.0, -1.0, 1.0],
            [-1.0, 1.0, -1.0], [1.0, 1.0, -1.0], [1.0, 1.0, 1.0], [-1.0, 1.0, 1.0],
        ].to_vec();
        compute_hull(&verts)
    }

    #[test]
    fn cube_flat_face_stable() {
        let mesh = cube_mesh();
        let hull = cube_hull();
        let stability = check_stability(&[0.0, 0.0, -1.0], &mesh, &hull);
        assert!(stability.stable, "Cube on face should be stable");
        assert!(stability.margin > 0.0);
        assert!(stability.contact_area > 0.0);
    }

    #[test]
    fn cube_vertex_down_low_contact() {
        let mesh = cube_mesh();
        let hull = cube_hull();
        let dir = [-0.57735, -0.57735, -0.57735];
        let stability = check_stability(&dir, &mesh, &hull);
        assert!(stability.contact_area < 0.1, "Cube on vertex should have tiny contact area");
    }

    #[test]
    fn horizontal_triangle_single() {
        let positions: Vec<f32> = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let mesh = precompute_mesh(&positions);
        let hull = compute_hull(&[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]);
        let stability = check_stability(&[0.0, 0.0, -1.0], &mesh, &hull);
        assert!(stability.stable);
        assert!(stability.contact_area > 0.0);
    }
}
