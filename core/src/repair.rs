//! STL mesh repair — remove defects that degrade orientation scoring.
//!
//! - Remove triangles with near-zero area (slivers)
//! - Weld near-duplicate vertices
//! - Remove duplicate triangles (identical vertex triples)
//! - No normal fix needed — `precompute_mesh` already recomputes normals from
//!   the cross product and ignores STL header normals.
//!
//! All functions operate on the flat soup arrays from `OriData`, keeping
//! positions (9 floats per triangle), normals (3 per triangle), and areas
//! (1 per triangle) in sync.

/// Default area threshold: triangles with area < `AREA_EPSILON * bbox_diag²`
/// are treated as slivers and removed.
const AREA_EPSILON: f32 = 1e-12;

/// Default vertex weld distance: vertices within `WELD_EPSILON * bbox_diag`
/// are merged.
const WELD_EPSILON: f32 = 1e-8;

/// Repair a triangle soup in-place. All three arrays are kept in sync:
/// positions\[9t\] + normals\[9t\] + areas\[t\] = triangle t.
///
/// Steps:
///   1. Weld near-duplicate vertices (re-index triangles)
///   2. Remove triangles whose cross-product area < threshold
///   3. Remove triangles with duplicate vertex triples
///   4. Update normals and areas to match surviving triangles
pub fn repair_mesh(positions: &mut Vec<f32>, normals: &mut Vec<f32>, areas: &mut Vec<f32>) {
    let tri_count = normals.len() / 3;
    if tri_count == 0 {
        return;
    }

    // Compute bbox for relative thresholds
    let (mut xmin, mut xmax) = (f32::INFINITY, f32::NEG_INFINITY);
    let (mut ymin, mut ymax) = (f32::INFINITY, f32::NEG_INFINITY);
    let (mut zmin, mut zmax) = (f32::INFINITY, f32::NEG_INFINITY);
    for i in 0..positions.len() / 3 {
        let p = &positions[i * 3..i * 3 + 3];
        if p[0] < xmin { xmin = p[0]; }
        if p[0] > xmax { xmax = p[0]; }
        if p[1] < ymin { ymin = p[1]; }
        if p[1] > ymax { ymax = p[1]; }
        if p[2] < zmin { zmin = p[2]; }
        if p[2] > zmax { zmax = p[2]; }
    }
    let diag = ((xmax - xmin).max(ymax - ymin).max(zmax - zmin)).max(1e-12);
    let area_thresh = AREA_EPSILON * diag * diag;
    let weld_dist = WELD_EPSILON * diag;

    // Step 1: weld vertices — map old vertex index → new vertex index
    let mut weld_map: Vec<usize> = (0..positions.len() / 3).collect();
    let weld_dist_sq = weld_dist * weld_dist;
    for i in 0..weld_map.len() {
        if weld_map[i] != i { continue; }
        let pi = &positions[i * 3..i * 3 + 3];
        for j in (i + 1)..weld_map.len() {
            let pj = &positions[j * 3..j * 3 + 3];
            let dx = pi[0] - pj[0];
            let dy = pi[1] - pj[1];
            let dz = pi[2] - pj[2];
            if dx * dx + dy * dy + dz * dz <= weld_dist_sq {
                weld_map[j] = i;
            }
        }
    }

    // Step 2 & 3: filter triangles — keep those with area > threshold
    // and no duplicate vertex triple.
    let old_tri_count = tri_count;
    let mut keep = vec![false; old_tri_count];
    let mut kept_count = 0usize;
    // Hash set for detecting duplicate triangles (sorted vertex indices)
    let mut seen = std::collections::HashSet::new();

    for t in 0..old_tri_count {
        let v0 = weld_map[t * 3];
        let v1 = weld_map[t * 3 + 1];
        let v2 = weld_map[t * 3 + 2];

        if v0 == v1 || v1 == v2 || v0 == v2 {
            continue; // degenerate after welding
        }

        // Area check (cross product)
        let p0 = &positions[v0 * 3..v0 * 3 + 3];
        let p1 = &positions[v1 * 3..v1 * 3 + 3];
        let p2 = &positions[v2 * 3..v2 * 3 + 3];

        let e1x = p1[0] - p0[0];
        let e1y = p1[1] - p0[1];
        let e1z = p1[2] - p0[2];
        let e2x = p2[0] - p0[0];
        let e2y = p2[1] - p0[1];
        let e2z = p2[2] - p0[2];

        let cx = e1y * e2z - e1z * e2y;
        let cy = e1z * e2x - e1x * e2z;
        let cz = e1x * e2y - e1y * e2x;
        let area = 0.5 * (cx * cx + cy * cy + cz * cz).sqrt();

        if area <= area_thresh {
            continue; // sliver
        }

        // Duplicate triangle check (sorted vertex indices)
        let mut key = [v0, v1, v2];
        key.sort_unstable();
        if !seen.insert(key) {
            continue; // duplicate
        }

        keep[t] = true;
        kept_count += 1;
    }

    // Step 4: compact — rebuild arrays in-place
    let mut write = 0usize;
    for t in 0..old_tri_count {
        if !keep[t] {
            continue;
        }
        // Copy positions (9 floats)
        let src_pos = t * 9;
        positions[write * 9] = positions[src_pos];
        positions[write * 9 + 1] = positions[src_pos + 1];
        positions[write * 9 + 2] = positions[src_pos + 2];
        positions[write * 9 + 3] = positions[src_pos + 3];
        positions[write * 9 + 4] = positions[src_pos + 4];
        positions[write * 9 + 5] = positions[src_pos + 5];
        positions[write * 9 + 6] = positions[src_pos + 6];
        positions[write * 9 + 7] = positions[src_pos + 7];
        positions[write * 9 + 8] = positions[src_pos + 8];

        normals[write * 3] = normals[t * 3];
        normals[write * 3 + 1] = normals[t * 3 + 1];
        normals[write * 3 + 2] = normals[t * 3 + 2];

        areas[write] = areas[t];

        write += 1;
    }

    positions.truncate(kept_count * 9);
    normals.truncate(kept_count * 3);
    areas.truncate(kept_count);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reconstruct_mesh;
    use crate::scoring;

    fn make_positions(tris: &[[[f32; 3]; 3]]) -> Vec<f32> {
        let mut v = Vec::with_capacity(tris.len() * 9);
        for tri in tris {
            for p in tri {
                v.push(p[0]);
                v.push(p[1]);
                v.push(p[2]);
            }
        }
        v
    }

    fn make_normals(tris: &[[[f32; 3]; 3]]) -> Vec<f32> {
        let mut n = Vec::with_capacity(tris.len() * 3);
        for tri in tris {
            let e1 = [tri[1][0] - tri[0][0], tri[1][1] - tri[0][1], tri[1][2] - tri[0][2]];
            let e2 = [tri[2][0] - tri[0][0], tri[2][1] - tri[0][1], tri[2][2] - tri[0][2]];
            let cx = e1[1] * e2[2] - e1[2] * e2[1];
            let cy = e1[2] * e2[0] - e1[0] * e2[2];
            let cz = e1[0] * e2[1] - e1[1] * e2[0];
            let len = (cx * cx + cy * cy + cz * cz).sqrt();
            if len > 0.0 {
                n.push(cx / len);
                n.push(cy / len);
                n.push(cz / len);
            } else {
                n.push(0.0);
                n.push(0.0);
                n.push(1.0);
            }
        }
        n
    }

    fn make_areas(tris: &[[[f32; 3]; 3]]) -> Vec<f32> {
        let mut a = Vec::with_capacity(tris.len());
        for tri in tris {
            let e1 = [tri[1][0] - tri[0][0], tri[1][1] - tri[0][1], tri[1][2] - tri[0][2]];
            let e2 = [tri[2][0] - tri[0][0], tri[2][1] - tri[0][1], tri[2][2] - tri[0][2]];
            let cx = e1[1] * e2[2] - e1[2] * e2[1];
            let cy = e1[2] * e2[0] - e1[0] * e2[2];
            let cz = e1[0] * e2[1] - e1[1] * e2[0];
            a.push(0.5 * (cx * cx + cy * cy + cz * cz).sqrt());
        }
        a
    }

    // ── Tests ────────────────────────────────────────────────────────────────

    #[test]
    fn repair_empty_mesh() {
        let mut pos = vec![];
        let mut norm = vec![];
        let mut area = vec![];
        repair_mesh(&mut pos, &mut norm, &mut area);
        assert!(pos.is_empty());
        assert!(norm.is_empty());
        assert!(area.is_empty());
    }

    #[test]
    fn repair_keeps_good_triangles() {
        // One clean triangle in XY plane, area = 0.5
        let tris = [[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]];
        let mut pos = make_positions(&tris);
        let mut norm = make_normals(&tris);
        let mut area = make_areas(&tris);
        repair_mesh(&mut pos, &mut norm, &mut area);
        assert_eq!(pos.len(), 9);
        assert_eq!(norm.len(), 3);
        assert_eq!(area.len(), 1);
        assert!((area[0] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn repair_removes_degenerate_after_welding() {
        // Two triangles sharing two vertices (welds to degenerate)
        let tris = [
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],  // copy
        ];
        let mut pos = make_positions(&tris);
        let mut norm = make_normals(&tris);
        let mut area = make_areas(&tris);
        repair_mesh(&mut pos, &mut norm, &mut area);
        // Both have same vertex positions, but after welding vertices are merged,
        // triangle 2 becomes degenerate (v0==v1==v2). Only 1 should survive.
        assert_eq!(area.len(), 1);
    }

    #[test]
    fn repair_removes_sliver_triangle() {
        // A normal triangle + a tiny sliver
        let tris = [
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],           // area 0.5
            [[0.0, 0.0, 0.0], [1e-7, 0.0, 0.0], [0.0, 1e-7, 0.0]],         // area ~5e-15
        ];
        let mut pos = make_positions(&tris);
        let mut norm = make_normals(&tris);
        let mut area = make_areas(&tris);
        repair_mesh(&mut pos, &mut norm, &mut area);
        assert_eq!(area.len(), 1, "sliver should be removed");
        assert!((area[0] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn repair_removes_duplicate_triangles() {
        let tris = [
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], // exact duplicate
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], // another exact duplicate
        ];
        let mut pos = make_positions(&tris);
        let mut norm = make_normals(&tris);
        let mut area = make_areas(&tris);
        repair_mesh(&mut pos, &mut norm, &mut area);
        assert_eq!(area.len(), 1, "duplicates should be removed");
    }

    #[test]
    fn repair_removes_duplicate_with_different_winding() {
        // Same vertices, different winding order — should still be detected as duplicate
        let tris = [
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            [[0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]], // same vertices, reversed winding
        ];
        let mut pos = make_positions(&tris);
        let mut norm = make_normals(&tris);
        let mut area = make_areas(&tris);
        repair_mesh(&mut pos, &mut norm, &mut area);
        assert_eq!(area.len(), 1, "same vertices different winding should be deduped");
    }

    #[test]
    fn repair_does_not_mutate_all_healthy_triangles() {
        // Two distinct healthy triangles
        let tris = [
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            [[0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [0.0, 1.0, 1.0]],
        ];
        let mut pos = make_positions(&tris);
        let mut norm = make_normals(&tris);
        let mut area = make_areas(&tris);
        let orig_norm = norm.clone();
        repair_mesh(&mut pos, &mut norm, &mut area);
        assert_eq!(area.len(), 2);
        assert_eq!(norm, orig_norm, "normals should not change for healthy triangles");
    }

    #[test]
    fn repair_scoring_with_repaired_mesh_is_stable() {
        // Two cubes: one has extra degenerate + duplicate triangles
        // After repair, scoring should match the clean cube.

        // Unit cube (12 triangles, clean)
        let clean_pos: Vec<f32> = vec![
            0.0,0.0,0.0,  1.0,0.0,0.0,  1.0,1.0,0.0,
            0.0,0.0,0.0,  1.0,1.0,0.0,  0.0,1.0,0.0,
            0.0,0.0,1.0,  1.0,0.0,1.0,  1.0,1.0,1.0,
            0.0,0.0,1.0,  1.0,1.0,1.0,  0.0,1.0,1.0,
            0.0,0.0,0.0,  1.0,0.0,1.0,  1.0,0.0,0.0,
            0.0,0.0,0.0,  0.0,0.0,1.0,  1.0,0.0,1.0,
            0.0,1.0,0.0,  1.0,1.0,0.0,  1.0,1.0,1.0,
            0.0,1.0,0.0,  1.0,1.0,1.0,  0.0,1.0,1.0,
            0.0,0.0,0.0,  0.0,1.0,0.0,  0.0,1.0,1.0,
            0.0,0.0,0.0,  0.0,1.0,1.0,  0.0,0.0,1.0,
            1.0,0.0,0.0,  1.0,1.0,0.0,  1.0,1.0,1.0,
            1.0,0.0,0.0,  1.0,1.0,1.0,  1.0,0.0,1.0,
        ];
        let clean_mesh = reconstruct_mesh(&clean_pos, &[0.0, 0.0, -1.0, 0.0, 0.0, -1.0,
            0.0, 0.0, 1.0, 0.0, 0.0, 1.0,
            0.0, -1.0, 0.0, 0.0, -1.0, 0.0,
            0.0, 1.0, 0.0, 0.0, 1.0, 0.0,
            -1.0, 0.0, 0.0, -1.0, 0.0, 0.0,
            1.0, 0.0, 0.0, 1.0, 0.0, 0.0,
        ], &[0.5; 12]);

        let dir = [0.0, -1.0, 0.0];
        let clean_overhang = scoring::score_candidate(&dir, &clean_mesh, 30.0);

        // Same cube with duplicates and slivers appended
        let mut dirty_pos = clean_pos.clone();
        let mut dirty_norm: Vec<f32> = vec![
            0.0, 0.0, -1.0, 0.0, 0.0, -1.0,
            0.0, 0.0, 1.0, 0.0, 0.0, 1.0,
            0.0, -1.0, 0.0, 0.0, -1.0, 0.0,
            0.0, 1.0, 0.0, 0.0, 1.0, 0.0,
            -1.0, 0.0, 0.0, -1.0, 0.0, 0.0,
            1.0, 0.0, 0.0, 1.0, 0.0, 0.0,
        ];
        let mut dirty_area: Vec<f32> = vec![0.5; 12];

        // Append a duplicate of the first triangle
        dirty_pos.extend_from_slice(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0]);
        dirty_norm.extend_from_slice(&[0.0, 0.0, -1.0]);
        dirty_area.push(0.5);

        // Append a tiny sliver
        dirty_pos.extend_from_slice(&[0.0, 0.0, 0.0, 1e-8, 0.0, 0.0, 0.0, 1e-8, 0.0]);
        dirty_norm.extend_from_slice(&[0.0, 0.0, 1.0]);
        dirty_area.push(5e-17);

        repair_mesh(&mut dirty_pos, &mut dirty_norm, &mut dirty_area);

        let dirty_mesh = reconstruct_mesh(&dirty_pos, &dirty_norm, &dirty_area);
        let repaired_overhang = scoring::score_candidate(&dir, &dirty_mesh, 30.0);

        assert!(
            (repaired_overhang - clean_overhang).abs() < 1e-6,
            "repaired mesh overhang ({}) should match clean mesh overhang ({})",
            repaired_overhang, clean_overhang
        );
    }
}
