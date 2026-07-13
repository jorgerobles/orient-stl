/// Yaw/quaternion helpers for orientation preview.
///
/// Faithful ports of TS `computeDefaultYaw`, `quaternionAlign`, `multiplyQuats`
/// from `web/src/compute.ts`. The full orientation quaternion convention is:
///   qFull = bbox_min_yaw(dir, mesh) * quaternion_align(dir, [0,-1,0])
/// i.e., align candidate dir to -Y first, then apply bbox-minimizing yaw.
/// This matches the LOCKED Phase 2 decision (STATE.md).

/// Quaternion that rotates unit vector `a` to align with unit vector `b`.
/// Both vectors are assumed unit-length (caller normalises).
/// Returns [w, x, y, z]. Port of TS `quaternionAlign` (lines 547-565).
///
/// Edge cases:
/// - dot > 0.9999 → identity [1, 0, 0, 0] (vectors already aligned)
/// - dot < -0.9999 → 180° about a perpendicular axis (w=0)
/// - Otherwise → half-angle axis construction
pub(crate) fn quaternion_align(a: &[f32; 3], b: &[f32; 3]) -> [f32; 4] {
    let dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    if dot > 0.9999 {
        return [1.0, 0.0, 0.0, 0.0];
    }
    if dot < -0.9999 {
        // 180° rotation about a perpendicular axis
        let axis = if a[0].abs() < 0.9 {
            // cross(a, [1, 0, 0])
            [0.0, -a[2], a[1]]
        } else {
            // cross(a, [0, 1, 0])
            [a[2], 0.0, -a[0]]
        };
        let al = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2])
            .sqrt()
            .max(1e-12);
        [0.0, axis[0] / al, axis[1] / al, axis[2] / al]
    } else {
        let axis = [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]; // cross(a, b)
        let al = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2])
            .sqrt()
            .max(1e-12);
        let naxis = [axis[0] / al, axis[1] / al, axis[2] / al];
        let half = dot.acos() / 2.0;
        let s = half.sin();
        [half.cos(), naxis[0] * s, naxis[1] * s, naxis[2] * s]
    }
}

/// Hamilton product a * b (applies b first, then a).
/// Port of TS `multiplyQuats` (lines 567-577) — exact formula, element by element.
pub(crate) fn multiply_quats(a: &[f32; 4], b: &[f32; 4]) -> [f32; 4] {
    [
        a[0] * b[0] - a[1] * b[1] - a[2] * b[2] - a[3] * b[3],
        a[0] * b[1] + a[1] * b[0] + a[2] * b[3] - a[3] * b[2],
        a[0] * b[2] - a[1] * b[3] + a[2] * b[0] + a[3] * b[1],
        a[0] * b[3] + a[1] * b[2] - a[2] * b[1] + a[3] * b[0],
    ]
}

/// Yaw-only quaternion that minimizes the XY bounding box of the mesh when
/// oriented along `dir`. Brute-force 180 yaw angles, picks smallest bbox area.
///
/// Port of TS `computeDefaultYaw` (lines 221-256). Renamed because:
/// 1) It only does yaw (not the full alignment quaternion).
/// 2) The old `candidates::compute_default_yaw` used a different convention
///    (Z-align, not -Y-align) — it is deprecated.
pub(crate) fn bbox_min_yaw(dir: &[f32; 3], mesh: &crate::mesh::MeshData) -> [f32; 4] {
    let dl = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
    if dl < 1e-8 {
        return [1.0, 0.0, 0.0, 0.0];
    }
    let dn = [dir[0] / dl, dir[1] / dl, dir[2] / dl];
    let up = [-dn[0], -dn[1], -dn[2]];

    // Perpendicular basis (same logic as stability.rs find_perpendicular).
    let x_axis = if up[0].abs() < 0.9 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let up_x0 = up[1] * x_axis[2] - up[2] * x_axis[1];
    let up_x1 = up[2] * x_axis[0] - up[0] * x_axis[2];
    let up_x2 = up[0] * x_axis[1] - up[1] * x_axis[0];
    let ux_len = (up_x0 * up_x0 + up_x1 * up_x1 + up_x2 * up_x2)
        .sqrt()
        .max(1e-12);
    let up_x = [up_x0 / ux_len, up_x1 / ux_len, up_x2 / ux_len];
    let up_y = [
        up[1] * up_x[2] - up[2] * up_x[1],
        up[2] * up_x[0] - up[0] * up_x[2],
        up[0] * up_x[1] - up[1] * up_x[0],
    ];

    // Project all vertices to 2D (u, v) plane perpendicular to dir.
    let pts2d: Vec<[f32; 2]> = mesh
        .vertices
        .iter()
        .map(|v| {
            [
                v[0] * up_x[0] + v[1] * up_x[1] + v[2] * up_x[2],
                v[0] * up_y[0] + v[1] * up_y[1] + v[2] * up_y[2],
            ]
        })
        .collect();

    // 2D convex hull of the projection.
    let hull = convex_hull_2d(&pts2d);

    // Brute-force 180 angles; pick the one minimizing bbox area.
    let mut best_angle = 0.0f32;
    let mut best_area = f32::INFINITY;
    for s in 0..180 {
        let angle = (s as f32 / 180.0) * std::f32::consts::PI;
        let (sa, ca) = angle.sin_cos();
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for &[px, py] in &hull {
            let rx = px * ca - py * sa;
            let ry = px * sa + py * ca;
            if rx < min_x {
                min_x = rx;
            }
            if rx > max_x {
                max_x = rx;
            }
            if ry < min_y {
                min_y = ry;
            }
            if ry > max_y {
                max_y = ry;
            }
        }
        let area = (max_x - min_x) * (max_y - min_y);
        if area < best_area {
            best_area = area;
            best_angle = angle;
        }
    }

    let _ = best_area;
    let half = best_angle / 2.0;
    let (hs, hc) = half.sin_cos();
    [hc, dn[0] * hs, dn[1] * hs, dn[2] * hs]
}

/// Full orientation quaternion = bbox_min_yaw × quaternion_align(dir, -Y).
///
/// Per LOCKED Phase 2 decision: align candidate dir to -Y first, then apply
/// bbox-minimizing yaw. This matches the deleted TS convention:
///   qFull = multiplyQuats(qYaw, qAlign(dir, [0,-1,0]))
pub fn full_quaternion(dir: &[f32; 3], mesh: &crate::mesh::MeshData) -> [f32; 4] {
    let q_yaw = bbox_min_yaw(dir, mesh);
    let q_align = quaternion_align(dir, &[0.0, -1.0, 0.0]);
    multiply_quats(&q_yaw, &q_align)
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// 2D convex hull via Andrew's monotone chain. Private helper for bbox_min_yaw.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Test-local helper: apply a quaternion rotation to a 3D vector.
    ///  v' = v + 2*w*(axis×v) + 2*(axis×(axis×v))
    fn apply_quat(q: &[f32; 4], v: [f32; 3]) -> [f32; 3] {
        let [w, x, y, z] = q;
        let uv = [
            y * v[2] - z * v[1],
            z * v[0] - x * v[2],
            x * v[1] - y * v[0],
        ];
        let uuv = [
            y * uv[2] - z * uv[1],
            z * uv[0] - x * uv[2],
            x * uv[1] - y * uv[0],
        ];
        [
            v[0] + 2.0 * (w * uv[0] + uuv[0]),
            v[1] + 2.0 * (w * uv[1] + uuv[1]),
            v[2] + 2.0 * (w * uv[2] + uuv[2]),
        ]
    }

    // -----------------------------------------------------------------------
    // quaternion_align tests
    // -----------------------------------------------------------------------

    #[test]
    fn quaternion_align_identity_when_aligned() {
        let a = [0.0, 0.0, 1.0];
        let b = [0.0, 0.0, 1.0];
        let q = quaternion_align(&a, &b);
        assert!((q[0] - 1.0).abs() < 1e-5, "w should be ~1.0, got {}", q[0]);
        assert!(q[1].abs() < 1e-5, "x should be ~0, got {}", q[1]);
        assert!(q[2].abs() < 1e-5, "y should be ~0, got {}", q[2]);
        assert!(q[3].abs() < 1e-5, "z should be ~0, got {}", q[3]);
    }

    #[test]
    fn quaternion_align_z_to_neg_y() {
        let a = [0.0, 0.0, -1.0];
        let b = [0.0, -1.0, 0.0];
        let q = quaternion_align(&a, &b);
        let rotated = apply_quat(&q, a);
        assert!(
            (rotated[0] - b[0]).abs() < 1e-5
                && (rotated[1] - b[1]).abs() < 1e-5
                && (rotated[2] - b[2]).abs() < 1e-5,
            "rotated [{:.4},{:.4},{:.4}] should ≈ [{:.4},{:.4},{:.4}]",
            rotated[0], rotated[1], rotated[2], b[0], b[1], b[2]
        );
    }

    #[test]
    fn quaternion_align_negation_180_axis() {
        let a = [0.0, 0.0, 1.0];
        let b = [0.0, 0.0, -1.0];
        let q = quaternion_align(&a, &b);
        assert!(
            q[0].abs() < 1e-4,
            "w should be ~0 for 180° rotation, got {}",
            q[0]
        );
        let norm_sq = q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3];
        assert!(
            (norm_sq - 1.0).abs() < 1e-4,
            "quaternion should be unit, norm² = {}",
            norm_sq
        );
    }

    // -----------------------------------------------------------------------
    // multiply_quats tests
    // -----------------------------------------------------------------------

    #[test]
    fn multiply_quats_identity_left() {
        let identity = [1.0, 0.0, 0.0, 0.0];
        let q = [0.7071_f32, 0.7071, 0.0, 0.0];
        let result = multiply_quats(&identity, &q);
        assert!(
            (result[0] - q[0]).abs() < 1e-4
                && (result[1] - q[1]).abs() < 1e-4
                && (result[2] - q[2]).abs() < 1e-4
                && (result[3] - q[3]).abs() < 1e-4,
            "identity * q should equal q, got [{:.4},{:.4},{:.4},{:.4}]",
            result[0], result[1], result[2], result[3]
        );
    }

    #[test]
    fn multiply_quats_inverse_yields_identity() {
        let q = [0.7071_f32, 0.5, 0.3, 0.4];
        let norm_sq: f32 = q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3];
        let len = norm_sq.sqrt();
        let qn = [q[0] / len, q[1] / len, q[2] / len, q[3] / len];
        let conj = [qn[0], -qn[1], -qn[2], -qn[3]];
        let result = multiply_quats(&qn, &conj);
        assert!(
            (result[0] - 1.0).abs() < 1e-4
                && result[1].abs() < 1e-4
                && result[2].abs() < 1e-4
                && result[3].abs() < 1e-4,
            "q * conj(q) should be identity, got [{:.4},{:.4},{:.4},{:.4}]",
            result[0], result[1], result[2], result[3]
        );
    }

    // -----------------------------------------------------------------------
    // bbox_min_yaw tests
    // -----------------------------------------------------------------------

    #[test]
    fn bbox_min_yaw_unit_square_best_angle() {
        use crate::mesh::precompute_mesh;

        let positions: Vec<f32> = vec![
            -1.0, -1.0, 0.0, 1.0, -1.0, 0.0, 1.0, 1.0, 0.0,
            -1.0, -1.0, 0.0, 1.0, 1.0, 0.0, -1.0, 1.0, 0.0,
        ];
        let mesh = precompute_mesh(&positions);
        let q = bbox_min_yaw(&[0.0, 0.0, -1.0], &mesh);
        assert!(
            (q[0] - 1.0).abs() < 1e-4,
            "bbox yaw should be near-identity for a square, w={}",
            q[0]
        );
    }

    // -----------------------------------------------------------------------
    // full_quaternion tests
    // -----------------------------------------------------------------------

    #[test]
    fn full_quaternion_unit_cube_dir_z_neg() {
        use crate::mesh::precompute_mesh;

        let positions: Vec<f32> = vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0,
            0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0,
            0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0,
            0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0,
            0.0, 1.0, 0.0, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0,
            0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0,
            0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0,
            1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0,
            1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 1.0,
        ];
        let mesh = precompute_mesh(&positions);
        let q = full_quaternion(&[0.0, 0.0, -1.0], &mesh);
        let expected: [f32; 4] = [0.7071_f32, -0.7071, 0.0, 0.0];
        assert!(
            (q[0] - expected[0]).abs() < 1e-3
                && (q[1] - expected[1]).abs() < 1e-3
                && (q[2] - expected[2]).abs() < 1e-3
                && (q[3] - expected[3]).abs() < 1e-3,
            "qFull expected [{:.4},{:.4},{:.4},{:.4}], got [{:.4},{:.4},{:.4},{:.4}]",
            expected[0], expected[1], expected[2], expected[3],
            q[0], q[1], q[2], q[3]
        );
    }
}
