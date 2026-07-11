use std::collections::HashMap;

const EPS: f32 = 1e-6;

pub(crate) struct ConvexHull {
    pub face_normals: Vec<[f32; 3]>,
}

#[derive(Clone)]
struct Face {
    v: [usize; 3],
    normal: [f32; 3],
}

pub(crate) fn compute_hull(vertices: &[[f32; 3]]) -> ConvexHull {
    let n = vertices.len();
    if n < 4 {
        return ConvexHull { face_normals: Vec::new() };
    }

    let (mut faces, interior, remaining) = match build_initial_simplex(vertices) {
        Some(r) => r,
        None => return ConvexHull { face_normals: Vec::new() },
    };

    let mut remaining = remaining;
    let mut max_iter = 100_000;
    while max_iter > 0 {
        max_iter -= 1;
        let found = find_outside_vertex(&remaining, &faces, vertices);
        let (idx_in_remaining, point_idx, visible) = match found {
            Some(v) => v,
            None => break,
        };
        remaining.swap_remove(idx_in_remaining);

        let mut edge_count: HashMap<[usize; 2], (usize, Option<[usize; 2]>)> = HashMap::new();
        for &fi in &visible {
            let f = &faces[fi];
            for &[a, b] in &[[f.v[0], f.v[1]], [f.v[1], f.v[2]], [f.v[2], f.v[0]]] {
                let key = if a < b { [a, b] } else { [b, a] };
                let entry = edge_count.entry(key).or_default();
                entry.0 += 1;
                if entry.1.is_none() {
                    entry.1 = Some([a, b]);
                }
            }
        }

        let mut horizon: Vec<[usize; 2]> = Vec::new();
        for &(count, orient) in edge_count.values() {
            if count == 1 {
                if let Some(edge) = orient {
                    horizon.push(edge);
                }
            }
        }

        let mut new_faces: Vec<Face> = Vec::new();
        for &[a, b] in &horizon {
            if let Some(mut n) = compute_normal(vertices[a], vertices[b], vertices[point_idx]) {
                let center = midpoint3(vertices[a], vertices[b], vertices[point_idx]);
                if dot(n, sub(interior, center)) > 0.0 {
                    n = [-n[0], -n[1], -n[2]];
                    new_faces.push(Face { v: [b, a, point_idx], normal: n });
                } else {
                    new_faces.push(Face { v: [a, b, point_idx], normal: n });
                }
            }
        }

        for fi in visible.iter().rev() {
            faces.swap_remove(*fi);
        }
        faces.extend(new_faces);
    }

    let face_normals = faces.into_iter().map(|f| f.normal).collect();
    ConvexHull { face_normals }
}

#[inline]
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[inline]
fn midpoint3(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    [(a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0, (a[2] + b[2] + c[2]) / 3.0]
}

fn signed_dist(point: &[f32; 3], face: &Face, vertices: &[[f32; 3]]) -> f32 {
    let v0 = vertices[face.v[0]];
    dot(*point, face.normal) - dot(v0, face.normal)
}

fn compute_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> Option<[f32; 3]> {
    let e1 = sub(b, a);
    let e2 = sub(c, a);
    let cx = e1[1] * e2[2] - e1[2] * e2[1];
    let cy = e1[2] * e2[0] - e1[0] * e2[2];
    let cz = e1[0] * e2[1] - e1[1] * e2[0];
    let len_sq = cx * cx + cy * cy + cz * cz;
    if len_sq <= EPS {
        return None;
    }
    let inv = 1.0 / len_sq.sqrt();
    Some([cx * inv, cy * inv, cz * inv])
}

fn find_outside_vertex(
    remaining: &[usize],
    faces: &[Face],
    vertices: &[[f32; 3]],
) -> Option<(usize, usize, Vec<usize>)> {
    for (ri, &point_idx) in remaining.iter().enumerate() {
        let point = &vertices[point_idx];
        let mut visible: Vec<usize> = Vec::new();
        for (fi, f) in faces.iter().enumerate() {
            if signed_dist(point, f, vertices) > EPS {
                visible.push(fi);
            }
        }
        if !visible.is_empty() {
            return Some((ri, point_idx, visible));
        }
    }
    None
}

fn build_initial_simplex(
    vertices: &[[f32; 3]],
) -> Option<(Vec<Face>, [f32; 3], Vec<usize>)> {
    let n = vertices.len();

    let idx0 = 0;
    let mut idx1 = 1;
    let mut max_d = 0.0;
    for i in 1..n {
        let d = dist_sq(vertices[idx0], vertices[i]);
        if d > max_d {
            max_d = d;
            idx1 = i;
        }
    }
    if max_d <= EPS {
        return None;
    }

    let mut idx2 = 0;
    max_d = 0.0;
    for i in 0..n {
        if i == idx0 || i == idx1 {
            continue;
        }
        let d = dist_to_line_sq(vertices[idx0], vertices[idx1], vertices[i]);
        if d > max_d {
            max_d = d;
            idx2 = i;
        }
    }
    if max_d <= EPS {
        return None;
    }

    let mut idx3 = 0;
    max_d = 0.0;
    if let Some(normal) = compute_normal(vertices[idx0], vertices[idx1], vertices[idx2]) {
        for i in 0..n {
            if i == idx0 || i == idx1 || i == idx2 {
                continue;
            }
            let d = dist_to_plane_sq(vertices[idx0], normal, vertices[i]);
            if d > max_d {
                max_d = d;
                idx3 = i;
            }
        }
    }
    if max_d <= EPS {
        return None;
    }

    let pts = [idx0, idx1, idx2, idx3];
    let interior = [
        (vertices[idx0][0] + vertices[idx1][0] + vertices[idx2][0] + vertices[idx3][0]) / 4.0,
        (vertices[idx0][1] + vertices[idx1][1] + vertices[idx2][1] + vertices[idx3][1]) / 4.0,
        (vertices[idx0][2] + vertices[idx1][2] + vertices[idx2][2] + vertices[idx3][2]) / 4.0,
    ];

    let mut faces = Vec::new();
    for &(i, j, k) in &[(0, 1, 2), (0, 3, 1), (1, 3, 2), (0, 2, 3)] {
        let a = vertices[pts[i]];
        let b = vertices[pts[j]];
        let c = vertices[pts[k]];
        if let Some(n) = compute_normal(a, b, c) {
            let center = midpoint3(a, b, c);
            if dot(n, sub(interior, center)) > 0.0 {
                faces.push(Face { v: [pts[k], pts[j], pts[i]], normal: n });
            } else {
                faces.push(Face { v: [pts[i], pts[j], pts[k]], normal: n });
            }
        }
    }

    if faces.len() < 4 {
        return None;
    }

    let tetra_set = [idx0, idx1, idx2, idx3];
    let remaining: Vec<usize> = (0..n).filter(|i| !tetra_set.contains(i)).collect();

    Some((faces, interior, remaining))
}

fn dist_sq(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    dx * dx + dy * dy + dz * dz
}

fn dist_to_line_sq(a: [f32; 3], b: [f32; 3], p: [f32; 3]) -> f32 {
    let ab = sub(b, a);
    let ap = sub(p, a);
    let ab_len_sq = dot(ab, ab);
    if ab_len_sq <= EPS {
        return dist_sq(a, p);
    }
    let t = dot(ap, ab) / ab_len_sq;
    let proj = [a[0] + t * ab[0], a[1] + t * ab[1], a[2] + t * ab[2]];
    dist_sq(proj, p)
}

fn dist_to_plane_sq(plane_point: [f32; 3], normal: [f32; 3], p: [f32; 3]) -> f32 {
    let d = dot(sub(p, plane_point), normal);
    d * d
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube_vertices() -> Vec<[f32; 3]> {
        let mut v = Vec::new();
        for x in &[-1.0, 1.0] {
            for y in &[-1.0, 1.0] {
                for z in &[-1.0, 1.0] {
                    v.push([*x, *y, *z]);
                }
            }
        }
        v
    }

    fn tetrahedron_vertices() -> Vec<[f32; 3]> {
        vec![
            [0.0, 0.0, 1.0],
            [0.0, 1.0, -1.0],
            [1.0, -1.0, -1.0],
            [-1.0, -1.0, -1.0],
        ]
    }

    #[test]
    fn tetrahedron_has_4_normals() {
        let hull = compute_hull(&tetrahedron_vertices());
        assert_eq!(hull.face_normals.len(), 4);
    }

    #[test]
    fn coplanar_points_no_crash() {
        let pts: Vec<[f32; 3]> = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let hull = compute_hull(&pts);
        assert!(hull.face_normals.is_empty());
    }

    #[test]
    fn cube_has_12_or_more_faces() {
        let hull = compute_hull(&cube_vertices());
        assert!(hull.face_normals.len() >= 12, "Expected 12+ faces for cube, got {}", hull.face_normals.len());
        for n in &hull.face_normals {
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            assert!((len - 1.0).abs() < 0.01, "Normal {:?} not unit", n);
        }
    }

    #[test]
    fn cube_normals_cover_all_axes() {
        let hull = compute_hull(&cube_vertices());
        let mut axes = vec![false; 6];
        for n in &hull.face_normals {
            let axis_aligned =
                (n[0].abs() > 0.99 && n[1].abs() < 0.1 && n[2].abs() < 0.1) ||
                (n[0].abs() < 0.1 && n[1].abs() > 0.99 && n[2].abs() < 0.1) ||
                (n[0].abs() < 0.1 && n[1].abs() < 0.1 && n[2].abs() > 0.99);
            if axis_aligned {
                for (ai, &val) in [1.0, -1.0].iter().enumerate() {
                    for d in 0..3 {
                        if (n[d] - val).abs() < 0.01 {
                            axes[ai * 3 + d] = true;
                        }
                    }
                }
            }
        }
        assert!(axes.iter().all(|&a| a), "Missing axis normals: {:?}. Full normals: {:?}",
            axes, hull.face_normals);
    }

    #[test]
    fn random_tetrahedron() {
        let hull = compute_hull(&[
            [10.0, 0.0, 0.0],
            [0.0, 10.0, 0.0],
            [0.0, 0.0, 10.0],
            [0.0, 0.0, 0.0],
        ]);
        assert_eq!(hull.face_normals.len(), 4);
    }
}
