use crate::hull::ConvexHull;

pub(crate) fn generate_candidates(hull: &ConvexHull) -> Vec<[f32; 3]> {
    hull.face_normals.clone()
}

pub(crate) fn deduplicate_directions(directions: &[[f32; 3]], angle_deg: f32) -> Vec<[f32; 3]> {
    if directions.is_empty() || angle_deg <= 0.0 {
        return directions.to_vec();
    }
    let cos_threshold = (angle_deg as f64 * std::f64::consts::PI / 180.0).cos() as f32;
    let mut result: Vec<[f32; 3]> = Vec::new();
    'outer: for &dir in directions {
        for &keep in &result {
            let dot = dir[0] * keep[0] + dir[1] * keep[1] + dir[2] * keep[2];
            if dot >= cos_threshold {
                continue 'outer;
            }
        }
        result.push(dir);
    }
    result
}

pub(crate) fn generate_fibonacci_sphere(n: usize) -> Vec<[f32; 3]> {
    let golden = (1.0 + 5.0_f32.sqrt()) * 0.5;
    let mut dirs = Vec::with_capacity(n);
    for i in 0..n {
        let theta = (1.0 - 2.0 * (i as f32 + 0.5) / n as f32).acos();
        let phi = 2.0 * std::f32::consts::PI * i as f32 / golden;
        dirs.push([
            theta.sin() * phi.cos(),
            theta.sin() * phi.sin(),
            theta.cos(),
        ]);
    }
    dirs
}

pub(crate) fn generate_hull_plus_sphere(
    hull: &ConvexHull,
    n: usize,
    dedupe_angle_deg: f32,
) -> Vec<[f32; 3]> {
    let hull_normals = hull.face_normals.clone();
    let deduped_hull = deduplicate_directions(&hull_normals, dedupe_angle_deg);
    let sphere = generate_fibonacci_sphere(n);
    let mut combined = deduped_hull.clone();
    let cos_threshold = (dedupe_angle_deg as f64 * std::f64::consts::PI / 180.0).cos() as f32;
    for &dir in &sphere {
        let mut keep = true;
        for &existing in &deduped_hull {
            let dot = dir[0] * existing[0] + dir[1] * existing[1] + dir[2] * existing[2];
            if dot >= cos_threshold {
                keep = false;
                break;
            }
        }
        if keep {
            combined.push(dir);
        }
    }
    combined
}

#[deprecated(note = "Use yaw::full_quaternion or yaw::bbox_min_yaw instead. This fn uses a Z-up convention; the new code uses the LOCKED -Y convention from Phase 2.")]
#[allow(dead_code)]
pub(crate) fn compute_default_yaw(
    direction: &[f32; 3],
    hull_vertices: &[[f32; 3]],
) -> [f32; 4] {
    let up = [0.0, 0.0, 1.0];
    let dn = [direction[0], direction[1], direction[2]];
    let dn_len = (dn[0] * dn[0] + dn[1] * dn[1] + dn[2] * dn[2]).sqrt();
    if dn_len < 1e-8 {
        return [1.0, 0.0, 0.0, 0.0];
    }
    let dn = [dn[0] / dn_len, dn[1] / dn_len, dn[2] / dn_len];

    let dot = up[0] * dn[0] + up[1] * dn[1] + up[2] * dn[2];
    let q = if (dot + 1.0).abs() < 1e-8 {
        [0.0, 1.0, 0.0, 0.0]
    } else if (1.0 - dot).abs() < 1e-8 {
        [1.0, 0.0, 0.0, 0.0]
    } else {
        let axis = [up[1] * dn[2] - up[2] * dn[1], up[2] * dn[0] - up[0] * dn[2], up[0] * dn[1] - up[1] * dn[0]];
        let axis_len = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
        let ax = [axis[0] / axis_len, axis[1] / axis_len, axis[2] / axis_len];
        let half = (dot * 0.5 + 0.5).sqrt();
        let s = 0.5 / half;
        [half, ax[0] * s, ax[1] * s, ax[2] * s]
    };

    let yaw = find_best_yaw(&q, &dn, hull_vertices);
    yaw
}

fn find_best_yaw(rot: &[f32; 4], down_dir: &[f32; 3], vertices: &[[f32; 3]]) -> [f32; 4] {
    let mut projected: Vec<[f32; 2]> = Vec::new();
    for &v in vertices {
        let local = rotate_point(v, rot, down_dir);
        projected.push([local[0], local[1]]);
    }
    if projected.is_empty() {
        return *rot;
    }

    let (min_area, best_angle) = rotating_calipers_bbox(&projected);
    let _ = min_area;

    let half = best_angle * 0.5;
    let (hs, hc) = half.sin_cos();
    let yaw_q = [hc, 0.0, 0.0, hs];

    quat_mul(rot, &yaw_q)
}

fn rotate_point(point: [f32; 3], rot: &[f32; 4], down: &[f32; 3]) -> [f32; 3] {
    let mut local = quat_rotate(rot, point);
    let dn_local = quat_rotate(rot, [0.0, 0.0, -1.0]);
    let align = [down[0] - dn_local[0], down[1] - dn_local[1], down[2] - dn_local[2]];
    if (align[0] * align[0] + align[1] * align[1] + align[2] * align[2]) > 0.01 {
        local = quat_rotate(rot, point);
    }
    local
}

fn quat_rotate(q: &[f32; 4], v: [f32; 3]) -> [f32; 3] {
    let [w, x, y, z] = q;
    let vx = v[0];
    let vy = v[1];
    let vz = v[2];
    let uv_x = y * vz - z * vy;
    let uv_y = z * vx - x * vz;
    let uv_z = x * vy - y * vx;
    let uuv_x = y * uv_z - z * uv_y;
    let uuv_y = z * uv_x - x * uv_z;
    let uuv_z = x * uv_y - y * uv_x;
    [
        vx + 2.0 * (w * uv_x + uuv_x),
        vy + 2.0 * (w * uv_y + uuv_y),
        vz + 2.0 * (w * uv_z + uuv_z),
    ]
}

fn quat_mul(a: &[f32; 4], b: &[f32; 4]) -> [f32; 4] {
    let [aw, ax, ay, az] = a;
    let [bw, bx, by, bz] = b;
    [
        aw * bw - ax * bx - ay * by - az * bz,
        aw * bx + ax * bw + ay * bz - az * by,
        aw * by - ax * bz + ay * bw + az * bx,
        aw * bz + ax * by - ay * bx + az * bw,
    ]
}

fn rotating_calipers_bbox(points: &[[f32; 2]]) -> (f32, f32) {
    let hull = convex_hull_2d(points);
    if hull.len() < 3 {
        let area = bbox_area(points, 0.0);
        return (area, 0.0);
    }

    let mut best_area = f32::MAX;
    let mut best_angle = 0.0;
    let m = hull.len();

    for i in 0..m {
        let j = (i + 1) % m;
        let dx = hull[j][0] - hull[i][0];
        let dy = hull[j][1] - hull[i][1];
        let angle = dy.atan2(dx);
        let area = bbox_area(points, angle);
        if area < best_area {
            best_area = area;
            best_angle = angle;
        }
    }
    (best_area, best_angle)
}

fn bbox_area(points: &[[f32; 2]], angle: f32) -> f32 {
    let (s, c) = angle.sin_cos();
    let mut min_u = f32::MAX;
    let mut max_u = f32::MIN;
    let mut min_v = f32::MAX;
    let mut max_v = f32::MIN;
    for &[x, y] in points {
        let u = x * c + y * s;
        let v = -x * s + y * c;
        if u < min_u { min_u = u; }
        if u > max_u { max_u = u; }
        if v < min_v { min_v = v; }
        if v > max_v { max_v = v; }
    }
    (max_u - min_u) * (max_v - min_v)
}

fn convex_hull_2d(points: &[[f32; 2]]) -> Vec<[f32; 2]> {
    if points.len() < 3 {
        return points.to_vec();
    }

    let mut pts: Vec<(f32, f32, usize)> = points
        .iter()
        .enumerate()
        .map(|(i, &[x, y])| (x, y, i))
        .collect();
    pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.1.partial_cmp(&b.1).unwrap()));

    let cross = |o: (f32, f32), a: (f32, f32), b: (f32, f32)| -> f32 {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    };

    let mut lower = Vec::new();
    for &p in &pts {
        let p2 = (p.0, p.1);
        while lower.len() >= 2 {
            let a = lower[lower.len() - 2];
            let b = lower[lower.len() - 1];
            if cross(a, b, p2) <= 0.0 {
                lower.pop();
            } else {
                break;
            }
        }
        lower.push(p2);
    }

    let mut upper = Vec::new();
    for &p in pts.iter().rev() {
        let p2 = (p.0, p.1);
        while upper.len() >= 2 {
            let a = upper[upper.len() - 2];
            let b = upper[upper.len() - 1];
            if cross(a, b, p2) <= 0.0 {
                upper.pop();
            } else {
                break;
            }
        }
        upper.push(p2);
    }

    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower.into_iter().map(|(x, y)| [x, y]).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_hull(normals: Vec<[f32; 3]>) -> ConvexHull {
        ConvexHull {
            face_normals: normals,
        }
    }

    #[test]
    fn generate_from_hull_normals() {
        let hull = make_hull(vec![[0.0, 0.0, 1.0], [0.0, 0.0, -1.0]]);
        let dirs = generate_candidates(&hull);
        assert_eq!(dirs.len(), 2);
    }

    #[test]
    fn dedupe_merges_close_directions() {
        let dirs = vec![[0.0, 0.0, 1.0], [0.0, 0.0349, 0.9994]];
        let deduped = deduplicate_directions(&dirs, 3.0);
        assert_eq!(deduped.len(), 1, "2° separation should merge at 3° threshold");
    }

    #[test]
    fn dedupe_keeps_different_directions() {
        let dirs = vec![[0.0, 0.0, 1.0], [0.0, 0.0698, 0.9976]];
        let deduped = deduplicate_directions(&dirs, 3.0);
        assert_eq!(deduped.len(), 2, "4° separation should NOT merge at 3° threshold");
    }

    #[test]
    fn test_compute_default_yaw() {
        let verts = vec![
            [-1.0, -1.0, 0.0],
            [1.0, -1.0, 0.0],
            [1.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0],
        ];
        let direction = [0.0, 0.0, -1.0];
        let q = compute_default_yaw(&direction, &verts);
        let rotated_up = quat_rotate(&q, [0.0, 0.0, 1.0]);
        let dot = direction[0]*rotated_up[0] + direction[1]*rotated_up[1] + direction[2]*rotated_up[2];
        assert!(dot > 0.99, "rotated up {:?} should align with direction {:?}", rotated_up, direction);
    }
}
