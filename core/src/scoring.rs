use crate::mesh::MeshData;

/// Build an orthonormal basis (e1, e2) perpendicular to `d` (assumed unit).
fn perpendicular_basis(d: &[f32; 3]) -> ([f32; 3], [f32; 3]) {
    let a = if d[0].abs() < 0.9 { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] };
    // e1 = normalize(d × a)
    let e1 = [
        d[1] * a[2] - d[2] * a[1],
        d[2] * a[0] - d[0] * a[2],
        d[0] * a[1] - d[1] * a[0],
    ];
    let l1 = (e1[0] * e1[0] + e1[1] * e1[1] + e1[2] * e1[2]).sqrt().max(1e-12);
    let e1 = [e1[0] / l1, e1[1] / l1, e1[2] / l1];
    // e2 = d × e1
    let e2 = [
        d[1] * e1[2] - d[2] * e1[1],
        d[2] * e1[0] - d[0] * e1[2],
        d[0] * e1[1] - d[1] * e1[0],
    ];
    (e1, e2)
}

/// H1 — area-weighted overhang penalty (current metric). Faces pointing along
/// `direction` beyond `critical_angle_deg` contribute area-weighted penalty.
pub(crate) fn score_candidate(
    direction: &[f32; 3],
    mesh: &MeshData,
    critical_angle_deg: f32,
) -> f32 {
    let theta = critical_angle_deg * std::f32::consts::PI / 180.0;
    let cos_critical = theta.cos();
    let mut total_penalty = 0.0f32;

    for i in 0..mesh.triangle_count {
        let cos_i = direction[0] * mesh.normals[i][0]
            + direction[1] * mesh.normals[i][1]
            + direction[2] * mesh.normals[i][2];
        if cos_i > cos_critical {
            let penalty = mesh.areas[i] * (cos_i - cos_critical);
            total_penalty += penalty;
        }
    }

    if !total_penalty.is_finite() {
        0.0
    } else {
        total_penalty
    }
}

/// H4 — footprint (shadow) area. Sum of each triangle's projected area onto the
/// plane whose normal is `direction`. For a shell mesh this is the total
/// projected area (overlapping projections overcounted, but monotonic in the
/// true shadow for convex hull normals — fine for ranking).
///
/// Cost: O(N), one dot product + abs + mul per triangle.
pub(crate) fn footprint_area(direction: &[f32; 3], mesh: &MeshData) -> f32 {
    let mut total = 0.0f32;
    for i in 0..mesh.triangle_count {
        let cos_i = direction[0] * mesh.normals[i][0]
            + direction[1] * mesh.normals[i][1]
            + direction[2] * mesh.normals[i][2];
        total += mesh.areas[i] * cos_i.abs();
    }
    if total.is_finite() {
        total
    } else {
        0.0
    }
}

/// H2 — max cross-section area (Z-histogram approximation). Bins each triangle
/// by its centroid projected onto `direction` into `bins` slices spanning the
/// mesh's extent along `direction`, sums projected area per bin, returns the
/// max bin. Proxy for peel force: the layer with the most material.
///
/// Cost: O(N), one centroid projection + bin lookup + area accumulate.
pub(crate) fn max_cross_section(direction: &[f32; 3], mesh: &MeshData, bins: usize) -> f32 {
    if mesh.triangle_count == 0 || bins == 0 {
        return 0.0;
    }
    // Find extent of triangle centroids along `direction`.
    let mut lo = f32::INFINITY;
    let mut hi = f32::NEG_INFINITY;
    let mut centroids_d = Vec::with_capacity(mesh.triangle_count);
    for i in 0..mesh.triangle_count {
        let v0 = &mesh.vertices[i * 3];
        let v1 = &mesh.vertices[i * 3 + 1];
        let v2 = &mesh.vertices[i * 3 + 2];
        let cd = (direction[0] * (v0[0] + v1[0] + v2[0])
            + direction[1] * (v0[1] + v1[1] + v2[1])
            + direction[2] * (v0[2] + v1[2] + v2[2]))
            / 3.0;
        centroids_d.push(cd);
        if cd < lo {
            lo = cd;
        }
        if cd > hi {
            hi = cd;
        }
    }
    let span = (hi - lo).max(1e-9);
    let scale = (bins as f32) / span;
    let mut hist = vec![0.0f32; bins];
    for i in 0..mesh.triangle_count {
        let mut b = ((centroids_d[i] - lo) * scale) as usize;
        if b >= bins {
            b = bins - 1;
        }
        let cos_i = direction[0] * mesh.normals[i][0]
            + direction[1] * mesh.normals[i][1]
            + direction[2] * mesh.normals[i][2];
        hist[b] += mesh.areas[i] * cos_i.abs();
    }
    let mut best = 0.0f32;
    for h in hist {
        if h > best {
            best = h;
        }
    }
    if best.is_finite() {
        best
    } else {
        0.0
    }
}

/// H5 — surface-quality (axis-misalignment) score. Port of PrusaSlicer's
/// `get_misalginment_score` (Rotfinder.cpp:88). For each face, sums the L1
/// norm of the normal in the orientation frame (dn, e1, e2):
///   area × (|n·dn| + |n·e1| + |n·e2|)
/// HIGHER = better. The L1 norm is minimised (=1) when a face aligns with a
/// single frame axis (big flat shelf/wall — prints poorly) and maximised (=√3)
/// when the face is diagonal to all three. PrusaSlicer maximises this.
/// Cost: O(N), three dot products + abs + mul per triangle.
pub(crate) fn misalignment_score(direction: &[f32; 3], mesh: &MeshData) -> f32 {
    let dl = (direction[0] * direction[0] + direction[1] * direction[1] + direction[2] * direction[2]).sqrt();
    if dl < 1e-12 {
        return 0.0;
    }
    let dn = [direction[0] / dl, direction[1] / dl, direction[2] / dl];
    let (e1, e2) = perpendicular_basis(&dn);
    let mut total = 0.0f32;
    for i in 0..mesh.triangle_count {
        let n = &mesh.normals[i];
        let align = (n[0] * dn[0] + n[1] * dn[1] + n[2] * dn[2]).abs()
            + (n[0] * e1[0] + n[1] * e1[1] + n[2] * e1[2]).abs()
            + (n[0] * e2[0] + n[1] * e2[1] + n[2] * e2[2]).abs();
        total += mesh.areas[i] * align;
    }
    if total.is_finite() {
        total
    } else {
        0.0
    }
}

/// H6 — print height (extent along `direction`). Mirrors PrusaSlicer's
/// `find_min_z_height_rotation` (Rotfinder.cpp:445) which minimises the rotated
/// bounding-box Z size. LOWER = better (faster print, fewer layers).
/// Cost: O(N), one dot product per vertex.
pub(crate) fn min_z_height(direction: &[f32; 3], mesh: &MeshData) -> f32 {
    if mesh.vertices.is_empty() {
        return 0.0;
    }
    let dl = (direction[0] * direction[0] + direction[1] * direction[1] + direction[2] * direction[2]).sqrt();
    if dl < 1e-12 {
        return 0.0;
    }
    let dn = [direction[0] / dl, direction[1] / dl, direction[2] / dl];
    let mut lo = f32::INFINITY;
    let mut hi = f32::NEG_INFINITY;
    for v in &mesh.vertices {
        let d = v[0] * dn[0] + v[1] * dn[1] + v[2] * dn[2];
        if d < lo {
            lo = d;
        }
        if d > hi {
            hi = d;
        }
    }
    let h = (hi - lo).abs();
    if h.is_finite() {
        h
    } else {
        0.0
    }
}

/// Composite score — raw component tuple for the harness to normalise and
/// combine. Covers H1 (overhang), H4 (footprint), H2 (max cross-section),
/// H5 (surface quality — maximise), and H6 (print height — minimise).
pub struct ScoreComponents {
    pub overhang: f32,
    pub footprint: f32,
    pub max_cross: f32,
    pub surface_quality: f32,
    pub height: f32,
}

pub fn score_components(
    direction: &[f32; 3],
    mesh: &MeshData,
    critical_angle_deg: f32,
    cross_bins: usize,
) -> ScoreComponents {
    ScoreComponents {
        overhang: score_candidate(direction, mesh, critical_angle_deg),
        footprint: footprint_area(direction, mesh),
        max_cross: max_cross_section(direction, mesh, cross_bins),
        surface_quality: misalignment_score(direction, mesh),
        height: min_z_height(direction, mesh),
    }
}

/// H11 — shadowed-overhang fraction. Builds a 2.5D height field (min `d`-value
/// per grid cell in the plane perpendicular to `direction`), then for each
/// overhang triangle checks whether its centroid sits near the bottom of its
/// column (clear path to the build plate) or is lifted above another part of
/// the mesh (shadowed → suction-cup risk, harder supports, trapped resin).
///
/// Returns the fraction of overhang AREA that is shadowed, in [0,1].
/// 0.0 = every red face has a clear shot at the floor; 1.0 = all shadowed.
///
/// `tol_frac` is the height tolerance as a fraction of the mesh's span along
/// `direction` (a triangle within tol_frac of its column's min counts as clear).
///
/// Cost: O(N) — two passes (build field, query overhangs).
pub fn shadowed_overhang_fraction(
    direction: &[f32; 3],
    mesh: &MeshData,
    critical_angle_deg: f32,
    grid_res: usize,
    tol_frac: f32,
) -> f32 {
    let tri = mesh.triangle_count;
    if tri == 0 || grid_res == 0 {
        return 0.0;
    }
    let dn = {
        let l = (direction[0] * direction[0] + direction[1] * direction[1]
            + direction[2] * direction[2])
        .sqrt();
        if l < 1e-12 {
            return 0.0;
        }
        [direction[0] / l, direction[1] / l, direction[2] / l]
    };
    let (e1, e2) = perpendicular_basis(&dn);

    // Project each triangle centroid → (u, v, h) where h = centroid·dn.
    let mut u = vec![0.0f32; tri];
    let mut v = vec![0.0f32; tri];
    let mut h = vec![0.0f32; tri];
    let mut u_min = f32::INFINITY;
    let mut u_max = f32::NEG_INFINITY;
    let mut v_min = f32::INFINITY;
    let mut v_max = f32::NEG_INFINITY;
    let mut h_min = f32::INFINITY;
    let mut h_max = f32::NEG_INFINITY;
    for i in 0..tri {
        let a = &mesh.vertices[i * 3];
        let b = &mesh.vertices[i * 3 + 1];
        let c = &mesh.vertices[i * 3 + 2];
        let cx = (a[0] + b[0] + c[0]) / 3.0;
        let cy = (a[1] + b[1] + c[1]) / 3.0;
        let cz = (a[2] + b[2] + c[2]) / 3.0;
        let uu = cx * e1[0] + cy * e1[1] + cz * e1[2];
        let vv = cx * e2[0] + cy * e2[1] + cz * e2[2];
        let hh = cx * dn[0] + cy * dn[1] + cz * dn[2];
        u[i] = uu;
        v[i] = vv;
        h[i] = hh;
        if uu < u_min { u_min = uu; }
        if uu > u_max { u_max = uu; }
        if vv < v_min { v_min = vv; }
        if vv > v_max { v_max = vv; }
        if hh < h_min { h_min = hh; }
        if hh > h_max { h_max = hh; }
    }
    let u_span = (u_max - u_min).max(1e-9);
    let v_span = (v_max - v_min).max(1e-9);
    let h_span = (h_max - h_min).max(1e-9);
    let tol = (tol_frac * h_span).max(1e-9);
    let u_scale = (grid_res as f32) / u_span;
    let v_scale = (grid_res as f32) / v_span;

    // Height field: min h per cell. f32::INFINITY = empty cell.
    // Built by rasterizing each triangle into every grid cell it covers
    // (barycentric containment of the cell centre), so a large floor triangle
    // registers in all columns it spans — not just its centroid's column.
    let mut field = vec![f32::INFINITY; grid_res * grid_res];
    for i in 0..tri {
        let a = &mesh.vertices[i * 3];
        let b = &mesh.vertices[i * 3 + 1];
        let c = &mesh.vertices[i * 3 + 2];
        // Project the 3 vertices to (u,v).
        let u0 = a[0] * e1[0] + a[1] * e1[1] + a[2] * e1[2];
        let v0 = a[0] * e2[0] + a[1] * e2[1] + a[2] * e2[2];
        let u1 = b[0] * e1[0] + b[1] * e1[1] + b[2] * e1[2];
        let v1 = b[0] * e2[0] + b[1] * e2[1] + b[2] * e2[2];
        let u2 = c[0] * e1[0] + c[1] * e1[1] + c[2] * e1[2];
        let v2 = c[0] * e2[0] + c[1] * e2[1] + c[2] * e2[2];
        let cu_min = (((u0.min(u1).min(u2)) - u_min) * u_scale).floor() as isize;
        let cu_max = (((u0.max(u1).max(u2)) - u_min) * u_scale).floor() as isize;
        let cv_min = (((v0.min(v1).min(v2)) - v_min) * v_scale).floor() as isize;
        let cv_max = (((v0.max(v1).max(v2)) - v_min) * v_scale).floor() as isize;
        let cu_lo = cu_min.max(0) as usize;
        let cu_hi = (cu_max as usize).min(grid_res - 1);
        let cv_lo = cv_min.max(0) as usize;
        let cv_hi = (cv_max as usize).min(grid_res - 1);
        if cu_lo > cu_hi || cv_lo > cv_hi {
            continue;
        }
        // Edge functions for barycentric containment of cell centres.
        let w0u = v1 - v2; let w0v = u2 - u1;
        let w1u = v2 - v0; let w1v = u0 - u2;
        let w2u = v0 - v1; let w2v = u1 - u0;
        let area = u0 * w0u + u1 * w1u + u2 * w2u; // = 2× signed triangle area in (u,v)
        if area.abs() < 1e-12 {
            continue;
        }
        let inv_area = 1.0 / area;
        let inv_u_scale = 1.0 / u_scale;
        let inv_v_scale = 1.0 / v_scale;
        for cv in cv_lo..=cv_hi {
            // Cell centre in world (u,v): cell edge + half a cell.
            let vc = v_min + (cv as f32 + 0.5) * inv_v_scale;
            for cu in cu_lo..=cu_hi {
                let uc = u_min + (cu as f32 + 0.5) * inv_u_scale;
                let b0 = (w0u * (uc - u2) + w0v * (vc - v2)) * inv_area;
                let b1 = (w1u * (uc - u2) + w1v * (vc - v2)) * inv_area;
                let b2 = 1.0 - b0 - b1;
                // Containment with a small margin so edges register.
                if b0 >= -0.02 && b1 >= -0.02 && b2 >= -0.02 {
                    let cell = cv * grid_res + cu;
                    if h[i] < field[cell] {
                        field[cell] = h[i];
                    }
                }
            }
        }
    }

    // Query overhang triangles.
    let cos_crit = (critical_angle_deg * std::f32::consts::PI / 180.0).cos();
    let mut over_area = 0.0f32;
    let mut shadow_area = 0.0f32;
    for i in 0..tri {
        let cos_i = dn[0] * mesh.normals[i][0]
            + dn[1] * mesh.normals[i][1]
            + dn[2] * mesh.normals[i][2];
        if cos_i <= cos_crit {
            continue;
        }
        over_area += mesh.areas[i];
        let mut cu = ((u[i] - u_min) * u_scale) as usize;
        if cu >= grid_res { cu = grid_res - 1; }
        let mut cv = ((v[i] - v_min) * v_scale) as usize;
        if cv >= grid_res { cv = grid_res - 1; }
        let cell = cv * grid_res + cu;
        let floor = field[cell];
        if floor.is_finite() && h[i] - floor > tol {
            shadow_area += mesh.areas[i];
        }
    }
    if over_area <= 0.0 {
        return 0.0;
    }
    let frac = shadow_area / over_area;
    if frac.is_finite() {
        frac.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::precompute_mesh;

    fn horizontal_and_45_mesh() -> MeshData {
        let positions: Vec<f32> = vec![
            -1.0, -1.0, 0.0,
            1.0, -1.0, 0.0,
            1.0, 1.0, 0.0,
            0.0, 0.0, 0.0,
            1.0, 0.0, 0.0,
            0.0, 1.0, 1.0,
        ];
        precompute_mesh(&positions)
    }

    #[test]
    fn horizontal_face_zero_penalty() {
        let mesh = horizontal_and_45_mesh();
        let penalty = score_candidate(&[0.0, 0.0, -1.0], &mesh, 30.0);
        assert_eq!(penalty, 0.0);
    }

    #[test]
    fn critical_angle_affects_penalty() {
        let mesh = horizontal_and_45_mesh();
        let p30 = score_candidate(&[0.0, 0.0, -1.0], &mesh, 30.0);
        let p50 = score_candidate(&[0.0, 0.0, -1.0], &mesh, 50.0);
        assert!(p30 >= p50, "Higher critical angle should give lower or equal penalty");
    }

    #[test]
    fn top_face_no_overhang() {
        let positions: Vec<f32> = vec![
            -1.0, -1.0, 0.0,
            1.0, -1.0, 0.0,
            1.0, 1.0, 0.0,
        ];
        let mesh = precompute_mesh(&positions);
        let penalty = score_candidate(&[0.0, 0.0, -1.0], &mesh, 30.0);
        assert_eq!(penalty, 0.0);
    }

    // ---- H4 footprint_area tests ----

    fn unit_square_xy() -> MeshData {
        // 1×1 square in XY plane (two triangles), normal +Z, total area 1.0
        let positions: Vec<f32> = vec![
            0.0, 0.0, 0.0,
            1.0, 0.0, 0.0,
            1.0, 1.0, 0.0,
            0.0, 0.0, 0.0,
            1.0, 1.0, 0.0,
            0.0, 1.0, 0.0,
        ];
        precompute_mesh(&positions)
    }

    #[test]
    fn footprint_face_on_is_full_area() {
        // Square in XY plane, project onto Z (direction = +Z) → full area = 1.0
        let mesh = unit_square_xy();
        let fp = footprint_area(&[0.0, 0.0, 1.0], &mesh);
        assert!((fp - 1.0).abs() < 1e-5, "face-on footprint should be 1.0, got {}", fp);
    }

    #[test]
    fn footprint_edge_on_is_near_zero() {
        // Square in XY plane, project onto X (direction = +X) → ~0 (edge-on)
        let mesh = unit_square_xy();
        let fp = footprint_area(&[1.0, 0.0, 0.0], &mesh);
        assert!(fp < 1e-5, "edge-on footprint should be ~0, got {}", fp);
    }

    #[test]
    fn footprint_at_45deg_is_reduced() {
        // Square in XY plane, project onto diagonal → reduced by |cos(45°)|
        let mesh = unit_square_xy();
        let inv = std::f32::consts::FRAC_1_SQRT_2;
        let fp = footprint_area(&[inv, 0.0, inv], &mesh);
        // total area 1.0, projected = 1.0 * |cos(45°)| ≈ 0.7071
        assert!((fp - inv).abs() < 1e-5, "45° footprint should be √2/2, got {}", fp);
    }

    // ---- H2 max_cross_section tests ----

    #[test]
    fn cross_section_flat_slab_concentrates_in_one_bin() {
        // A flat slab in XY: all triangle centroids have d≈0 along Z → one bin
        // holds everything, max ≈ total projected area.
        let mesh = unit_square_xy();
        let mx = max_cross_section(&[0.0, 0.0, 1.0], &mesh, 8);
        assert!((mx - 1.0).abs() < 1e-5, "flat slab max cross ≈ 1.0, got {}", mx);
    }

    #[test]
    fn cross_section_spread_shape_has_smaller_max_than_slab() {
        // Two squares at z=0 and z=1 (a box-like shell). Along Z the centroids
        // spread across two bins, so each bin holds ~half → max < total.
        let positions: Vec<f32> = vec![
            // bottom square z=0
            0.0, 0.0, 0.0,  1.0, 0.0, 0.0,  1.0, 1.0, 0.0,
            0.0, 0.0, 0.0,  1.0, 1.0, 0.0,  0.0, 1.0, 0.0,
            // top square z=1
            0.0, 0.0, 1.0,  1.0, 0.0, 1.0,  1.0, 1.0, 1.0,
            0.0, 0.0, 1.0,  1.0, 1.0, 1.0,  0.0, 1.0, 1.0,
        ];
        let mesh = precompute_mesh(&positions);
        let mx = max_cross_section(&[0.0, 0.0, 1.0], &mesh, 8);
        // two separated layers → max bin ≈ 1.0 (one square), not 2.0
        assert!(mx < 1.5, "spread shape max should be < 1.5, got {}", mx);
    }

    #[test]
    fn cross_section_empty_mesh_returns_zero() {
        let mesh = precompute_mesh(&[]);
        assert_eq!(max_cross_section(&[0.0, 0.0, 1.0], &mesh, 8), 0.0);
    }

    // ---- H5 misalignment_score tests ----

    #[test]
    fn misalignment_face_on_is_area_only() {
        // Unit square XY, dir=+Z: |n·dn|=1, |n·e1|=|n·e2|=0 → L1=1 per face.
        // Score = 0.5*1 + 0.5*1 = 1.0 (lower bound for this mesh).
        let mesh = unit_square_xy();
        let s = misalignment_score(&[0.0, 0.0, 1.0], &mesh);
        assert!((s - 1.0).abs() < 1e-5, "face-on misalignment = total area = 1.0, got {}", s);
    }

    #[test]
    fn misalignment_diagonal_dir_is_higher() {
        // Same face, dir=(1,1,1): |n·dn|=1/√3, |n·e1|+|n·e2|>0 → L1>1 → higher score.
        let mesh = unit_square_xy();
        let aligned = misalignment_score(&[0.0, 0.0, 1.0], &mesh);
        let diagonal = misalignment_score(&[1.0, 1.0, 1.0], &mesh);
        assert!(diagonal > aligned, "diagonal dir should score higher, got {} vs {}", diagonal, aligned);
    }

    #[test]
    fn misalignment_zero_direction_returns_zero() {
        let mesh = unit_square_xy();
        assert_eq!(misalignment_score(&[0.0, 0.0, 0.0], &mesh), 0.0);
    }

    #[test]
    fn misalignment_empty_mesh_returns_zero() {
        let mesh = precompute_mesh(&[]);
        assert_eq!(misalignment_score(&[0.0, 0.0, 1.0], &mesh), 0.0);
    }

    // ---- H6 min_z_height tests ----

    #[test]
    fn height_flat_slab_is_zero() {
        // Unit square at z=0, dir=+Z → extent along Z = 0.
        let mesh = unit_square_xy();
        let h = min_z_height(&[0.0, 0.0, 1.0], &mesh);
        assert!(h < 1e-5, "flat slab height should be ~0, got {}", h);
    }

    #[test]
    fn height_two_layer_box_is_layer_gap() {
        // Two squares at z=0 and z=1 → extent along Z = 1.
        let positions: Vec<f32> = vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0,
            0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0,
        ];
        let mesh = precompute_mesh(&positions);
        let h = min_z_height(&[0.0, 0.0, 1.0], &mesh);
        assert!((h - 1.0).abs() < 1e-5, "two-layer height should be 1.0, got {}", h);
    }

    #[test]
    fn height_empty_mesh_returns_zero() {
        let mesh = precompute_mesh(&[]);
        assert_eq!(min_z_height(&[0.0, 0.0, 1.0], &mesh), 0.0);
    }

    // ---- H11 shadowed_overhang_fraction tests ----

    #[test]
    fn shadowed_no_overhang_returns_zero() {
        // A single horizontal triangle facing +Z, direction = +Z → its normal
        // points UP along dir → cos_i = +1 > cosCrit → it IS overhang (face
        // pointing along down-direction). Wait: dir=+Z, normal=+Z, dot=+1>cosCrit
        // → overhang. Centroid is at the bottom (only triangle) → clear path.
        let mesh = unit_square_xy();
        let frac = shadowed_overhang_fraction(&[0.0, 0.0, 1.0], &mesh, 30.0, 16, 0.02);
        // The square IS the bottom surface → not shadowed → 0.0
        assert!(frac < 0.05, "lone bottom face should be clear, got {}", frac);
    }

    #[test]
    fn shadowed_overhang_above_a_floor_is_detected() {
        // A "ceiling" overhang triangle floating above a "floor" triangle in the
        // same XY column. Both face +Z. The ceiling is shadowed by the floor.
        // Floor at z=0 (big), ceiling at z=5 (small, directly above).
        let positions: Vec<f32> = vec![
            // floor: large square centred at origin, z=0
            -10.0, -10.0, 0.0,  10.0, -10.0, 0.0,  10.0, 10.0, 0.0,
            -10.0, -10.0, 0.0,  10.0, 10.0, 0.0,   -10.0, 10.0, 0.0,
            // ceiling: small square centred at origin, z=5 (shadowed by floor)
            -1.0, -1.0, 5.0,  1.0, -1.0, 5.0,  1.0, 1.0, 5.0,
            -1.0, -1.0, 5.0,  1.0, 1.0, 5.0,   -1.0, 1.0, 5.0,
        ];
        let mesh = precompute_mesh(&positions);
        let frac = shadowed_overhang_fraction(&[0.0, 0.0, 1.0], &mesh, 30.0, 16, 0.02);
        // dir=+Z: both layers face +Z (dot=1>cosCrit) → both overhang. The
        // ceiling (z=5) is shadowed by the floor (z=0). floor area=200, ceiling=2.
        // shadowed fraction = ceiling_area / (floor+ceiling) = 2/202 ≈ 0.0099.
        assert!(frac > 0.0 && frac < 0.05, "expected small shadowed fraction, got {}", frac);
    }

    #[test]
    fn shadowed_clear_overhang_above_empty_space_is_not_shadowed() {
        // Two overhang triangles in DIFFERENT XY columns → neither shadows the
        // other. Both have a clear path to their own column's floor.
        let positions: Vec<f32> = vec![
            // square A centred at (-10, 0), z=0
            -11.0, -1.0, 0.0,  -9.0, -1.0, 0.0,  -9.0, 1.0, 0.0,
            -11.0, -1.0, 0.0,  -9.0, 1.0, 0.0,   -11.0, 1.0, 0.0,
            // square B centred at (+10, 0), z=0
            9.0, -1.0, 0.0,  11.0, -1.0, 0.0,  11.0, 1.0, 0.0,
            9.0, -1.0, 0.0,  11.0, 1.0, 0.0,   9.0, 1.0, 0.0,
        ];
        let mesh = precompute_mesh(&positions);
        let frac = shadowed_overhang_fraction(&[0.0, 0.0, 1.0], &mesh, 30.0, 16, 0.02);
        assert!(frac < 0.05, "separate columns should not shadow, got {}", frac);
    }

    #[test]
    fn shadowed_empty_mesh_returns_zero() {
        let mesh = precompute_mesh(&[]);
        assert_eq!(shadowed_overhang_fraction(&[0.0, 0.0, 1.0], &mesh, 30.0, 16, 0.02), 0.0);
    }
}
