use std::collections::{HashMap, HashSet, VecDeque};

/// Remove duplicate triangles from a triangle-soup position array.
/// Returns the number of triangles removed.
/// Operates O(n) — hashes the three vertex positions (sorted for winding
/// normalization) for each triangle.
pub fn repair_mesh(positions: &mut Vec<f32>) -> u32 {
    let n = positions.len() / 9; // 3 vertices × 3 coords
    if n < 2 {
        return 0;
    }

    let mut seen: HashSet<u64> = HashSet::with_capacity(n);
    let mut write_idx = 0;
    let mut removed = 0u32;

    for i in 0..n {
        let base = i * 9;
        let mut tri = [
            (positions[base], positions[base + 1], positions[base + 2]),
            (positions[base + 3], positions[base + 4], positions[base + 5]),
            (positions[base + 6], positions[base + 7], positions[base + 8]),
        ];
        // Canonicalise winding: sort vertices by (x, y, z)
        tri.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    a.1.partial_cmp(&b.1)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| {
                    a.2.partial_cmp(&b.2)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });

        // Hash the sorted positions into a 64-bit key
        let key = hash_tri(&tri);
        if !seen.insert(key) {
            removed += 1;
            continue;
        }

        if write_idx != i {
            let dst = write_idx * 9;
            positions.copy_within(base..base + 9, dst);
        }
        write_idx += 1;
    }

    if removed > 0 {
        positions.truncate(write_idx * 9);
    }
    removed
}

/// Normalize triangle winding by propagating orientation through shared edges.
/// Builds an edge→triangle map, then BFS across each connected component,
/// flipping triangles whose edge direction is inconsistent with their neighbor.
/// After propagation, determines absolute orientation per component via
/// centroid voting (most faces should point outward).
/// Returns the number of triangles flipped.
pub fn normalize_winding(positions: &mut Vec<f32>) -> u32 {
    let n = positions.len() / 9;
    if n < 2 {
        return 0;
    }

    let mut edge_map: HashMap<u64, Vec<(usize, u8)>> = HashMap::new();
    for i in 0..n {
        let base = i * 9;
        for e in 0..3u8 {
            let a_off = e as usize * 3;
            let b_off = ((e as usize + 1) % 3) * 3;
            let ax = positions[base + a_off];
            let ay = positions[base + a_off + 1];
            let az = positions[base + a_off + 2];
            let bx = positions[base + b_off];
            let by = positions[base + b_off + 1];
            let bz = positions[base + b_off + 2];
            if ax == bx && ay == by && az == bz {
                continue;
            }
            let key = edge_hash(ax, ay, az, bx, by, bz);
            edge_map.entry(key).or_default().push((i, e));
        }
    }

    let mut should_flip = vec![false; n];
    let mut visited = vec![false; n];
    // Per-component orientation: collect tris, then vote via centroid
    let mut components: Vec<Vec<usize>> = Vec::new();

    for seed in 0..n {
        if visited[seed] {
            continue;
        }
        let mut queue = VecDeque::new();
        visited[seed] = true;
        queue.push_back(seed);
        let mut comp = vec![seed];

        while let Some(tri) = queue.pop_front() {
            let base = tri * 9;
            for e in 0..3u8 {
                let a_off = e as usize * 3;
                let b_off = ((e as usize + 1) % 3) * 3;
                let ax = positions[base + a_off];
                let ay = positions[base + a_off + 1];
                let az = positions[base + a_off + 2];
                let bx = positions[base + b_off];
                let by = positions[base + b_off + 1];
                let bz = positions[base + b_off + 2];
                if ax == bx && ay == by && az == bz {
                    continue;
                }
                let key = edge_hash(ax, ay, az, bx, by, bz);

                if let Some(neighbors) = edge_map.get(&key) {
                    if neighbors.len() != 2 {
                        continue;
                    }
                    let neighbor_entry = neighbors.iter().find(|&&(t, _)| t != tri);
                    let &(neighbor, n_edge) = match neighbor_entry {
                        Some(e) => e,
                        None => continue,
                    };
                    if visited[neighbor] {
                        continue;
                    }

                    // Edge direction in current triangle (effective, considering flip)
                    let (tri_sx, tri_sy, tri_sz, tri_ex, tri_ey, tri_ez) =
                        if should_flip[tri] {
                            (bx, by, bz, ax, ay, az)
                        } else {
                            (ax, ay, az, bx, by, bz)
                        };

                    // Edge direction in neighbor (effective, considering its flip state)
                    let n_base = neighbor * 9;
                    let na_off = n_edge as usize * 3;
                    let nb_off = ((n_edge as usize + 1) % 3) * 3;
                    let n_ax = positions[n_base + na_off];
                    let n_ay = positions[n_base + na_off + 1];
                    let n_az = positions[n_base + na_off + 2];
                    let n_bx = positions[n_base + nb_off];
                    let n_by = positions[n_base + nb_off + 1];
                    let n_bz = positions[n_base + nb_off + 2];

                    let (n_sx, n_sy, n_sz, n_ex, n_ey, n_ez) = if should_flip[neighbor] {
                        (n_bx, n_by, n_bz, n_ax, n_ay, n_az)
                    } else {
                        (n_ax, n_ay, n_az, n_bx, n_by, n_bz)
                    };

                    // Consistent if edges run opposite directions:
                    // tri_start == neighbor_end AND tri_end == neighbor_start
                    let consistent = tri_sx == n_ex
                        && tri_sy == n_ey
                        && tri_sz == n_ez
                        && tri_ex == n_sx
                        && tri_ey == n_sy
                        && tri_ez == n_sz;

                    if !consistent {
                        should_flip[neighbor] = !should_flip[neighbor];
                    }

                    visited[neighbor] = true;
                    queue.push_back(neighbor);
                    comp.push(neighbor);
                }
            }
        }
        components.push(comp);
    }

    // Per-component absolute orientation via centroid voting.
    // Only for components with >= 4 triangles — below that the centroid
    // is too close to the surface and gives unreliable results.
    // BFS already ensures internal consistency within each component.
    for comp in &components {
        if comp.len() < 4 {
            continue;
        }
        // Compute component centroid
        let (mut cx, mut cy, mut cz) = (0.0f64, 0.0f64, 0.0f64);
        let mut verts = 0u64;
        for &tri in comp {
            let base = tri * 9;
            for j in 0..3 {
                let voff = j * 3;
                cx += positions[base + voff] as f64;
                cy += positions[base + voff + 1] as f64;
                cz += positions[base + voff + 2] as f64;
            }
            verts += 3;
        }
        if verts == 0 {
            continue;
        }
        let cx = cx / verts as f64;
        let cy = cy / verts as f64;
        let cz = cz / verts as f64;

        let mut outward_votes = 0i64;
        for &tri in comp {
            let base = tri * 9;
            let v1 = [positions[base], positions[base + 1], positions[base + 2]];
            let v2 = [positions[base + 3], positions[base + 4], positions[base + 5]];
            let v3 = [positions[base + 6], positions[base + 7], positions[base + 8]];
            let e1x = v2[0] - v1[0];
            let e1y = v2[1] - v1[1];
            let e1z = v2[2] - v1[2];
            let e2x = v3[0] - v1[0];
            let e2y = v3[1] - v1[1];
            let e2z = v3[2] - v1[2];
            let nx = e1y * e2z - e1z * e2y;
            let ny = e1z * e2x - e1x * e2z;
            let nz = e1x * e2y - e1y * e2x;
            let len_sq = nx * nx + ny * ny + nz * nz;
            if len_sq <= f32::EPSILON {
                continue;
            }
            let tc_x = (v1[0] + v2[0] + v3[0]) / 3.0;
            let tc_y = (v1[1] + v2[1] + v3[1]) / 3.0;
            let tc_z = (v1[2] + v2[2] + v3[2]) / 3.0;
            let dx = tc_x - cx as f32;
            let dy = tc_y - cy as f32;
            let dz = tc_z - cz as f32;
            let (eff_nx, eff_ny, eff_nz) = if should_flip[tri] {
                (-nx, -ny, -nz)
            } else {
                (nx, ny, nz)
            };
            if eff_nx * dx + eff_ny * dy + eff_nz * dz >= 0.0 {
                outward_votes += 1;
            } else {
                outward_votes -= 1;
            }
        }

        if outward_votes < 0 {
            for &tri in comp {
                should_flip[tri] = !should_flip[tri];
            }
        }
    }

    // Apply flips
    let mut flipped = 0u32;
    for i in 0..n {
        if should_flip[i] {
            let base = i * 9;
            positions.swap(base + 3, base + 6);
            positions.swap(base + 4, base + 7);
            positions.swap(base + 5, base + 8);
            flipped += 1;
        }
    }
    flipped
}

/// Canonical hash for an edge (direction-independent).
/// Sorts the two vertices by bitwise comparison, then FNV-1a of the 24 bytes.
fn edge_hash(ax: f32, ay: f32, az: f32, bx: f32, by: f32, bz: f32) -> u64 {
    let a_bits = (ax.to_bits(), ay.to_bits(), az.to_bits());
    let b_bits = (bx.to_bits(), by.to_bits(), bz.to_bits());
    let (x1, y1, z1, x2, y2, z2) = if a_bits < b_bits {
        (ax, ay, az, bx, by, bz)
    } else {
        (bx, by, bz, ax, ay, az)
    };
    let mut h = 14695981039346656037u64;
    for &coord in &[x1, y1, z1, x2, y2, z2] {
        for byte in coord.to_bits().to_le_bytes() {
            h ^= byte as u64;
            h = h.wrapping_mul(1099511628211);
        }
    }
    h
}

fn hash_tri(tri: &[(f32, f32, f32); 3]) -> u64 {
    // Mix each vertex with FNV-1a-like hashing
    let mut h = 14695981039346656037u64;
    for v in tri {
        let bytes = &[
            v.0.to_bits().to_le_bytes(),
            v.1.to_bits().to_le_bytes(),
            v.2.to_bits().to_le_bytes(),
        ];
        for b in bytes.iter().flatten() {
            h ^= *b as u64;
            h = h.wrapping_mul(1099511628211);
        }
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repair_empty_mesh() {
        let mut p = Vec::new();
        assert_eq!(repair_mesh(&mut p), 0);
    }

    #[test]
    fn repair_removes_duplicate_triangles() {
        let mut p = vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
        ];
        assert_eq!(repair_mesh(&mut p), 1);
        assert_eq!(p.len(), 9);
    }

    #[test]
    fn repair_removes_duplicate_with_different_winding() {
        // Same triangle, reversed winding
        let mut p = vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
            0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ];
        assert_eq!(repair_mesh(&mut p), 1);
        assert_eq!(p.len(), 9);
    }

    #[test]
    fn repair_keeps_unique_triangles() {
        let mut p = vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
            1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0,
        ];
        assert_eq!(repair_mesh(&mut p), 0);
        assert_eq!(p.len(), 18);
    }

    #[test]
    fn repair_no_collisions_simple() {
        // Upright and upside-down triangles share same vertices with
        // different positions, should NOT collide
        let mut p = vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        assert_eq!(repair_mesh(&mut p), 0);
        assert_eq!(p.len(), 18);
    }

    // ─── normalize_winding tests (edge-adjacency propagation) ──

    #[test]
    fn normalize_winding_empty() {
        let mut p: Vec<f32> = Vec::new();
        assert_eq!(normalize_winding(&mut p), 0);
    }

    #[test]
    fn normalize_winding_single_triangle() {
        // Single triangle, no shared edges → no flip
        let mut p = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        assert_eq!(normalize_winding(&mut p), 0);
    }

    #[test]
    fn normalize_winding_two_triangles_consistent() {
        // Two triangles sharing an edge with correct winding
        // Tri A: (0,0,0) (1,0,0) (0,1,0)  → normal +Z
        // Tri B: (1,0,0) (0,0,0) (0,0,1)  → shares edge (0,0,0)-(1,0,0),
        //        should have opposite edge direction: (1,0,0)→(0,0,0)
        //        Which it does: (1,0,0) (0,0,0) — correct!
        let mut p = vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
            1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let flips = normalize_winding(&mut p);
        assert_eq!(flips, 0);
    }

    #[test]
    fn normalize_winding_two_triangles_inverted() {
        // Two triangles sharing an edge with INCONSISTENT winding
        // Tri A: (0,0,0) (1,0,0) (0,1,0)  → normal +Z, edge (0,0,0)→(1,0,0)
        // Tri B: (0,0,0) (1,0,0) (0,0,1)  → shares edge (0,0,0)-(1,0,0)
        //        but edge direction is (0,0,0)→(1,0,0), SAME as Tri A
        //        → winding is inconsistent, should be flipped
        let mut p = vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let flips = normalize_winding(&mut p);
        assert_eq!(flips, 1, "Tri B should be flipped");
        // Tri B now should be (0,0,0) (0,0,1) (1,0,0)
        assert!((p[9 + 3] - 0.0).abs() < 1e-6);
        assert!((p[9 + 4] - 0.0).abs() < 1e-6);
        assert!((p[9 + 5] - 1.0).abs() < 1e-6);
        assert!((p[9 + 6] - 1.0).abs() < 1e-6);
        assert!((p[9 + 7] - 0.0).abs() < 1e-6);
        assert!((p[9 + 8] - 0.0).abs() < 1e-6);
    }

    #[test]
    fn normalize_winding_thin_shell() {
        // A thin shell: two triangles back-to-back sharing an edge, both
        // have winding that gives normal +Z. Tri B's winding should be flipped
        // so its normal points -Z (outward for the back face).
        let mut p = vec![
            // Front face: (0,0,0) (2,0,0) (0,2,0) → normal +Z
            0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 2.0, 0.0,
            // Back face: (0,0,0) (2,0,0) (0,2,0) → same winding, normal +Z
            // After orientation vote: this face should point -Z (away from centroid)
            0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 2.0, 0.0,
        ];
        // After dedup, they'd be the same triangle. But for winding norm,
        // they share the same vertices so edge hash will match.
        // With exact f32 equality, both triangles have identical edges.
        // The edge_map will have 3 entries, each with 2 triangles.
        // Non-manifold: neighbors.len() == 2 for each BUT both are the same
        // two triangles. So find(|&(t,_)| t != tri) returns the other one.
        //
        // BFS from seed 0: visit tri 0. Process edges:
        //   Each edge has [tri0, tri1] in the map.
        //   Check consistency: tri0's edge direction vs tri1's edge direction
        //   Tri0: (0,0,0)→(2,0,0), Tri1: (0,0,0)→(2,0,0) → SAME direction
        //   → inconsistent → flip tri1.
        //   After flip, tri1 becomes (0,0,0) (0,2,0) (2,0,0)
        //   Edge (0,0,0)→(2,0,0) in tri0, (2,0,0)→(0,0,0) in tri1 → consistent.
        //   Edge (2,0,0)→(0,2,0) ... wait, after flip the edges change.
        //   Let me verify: after flip:
        //     tri1 = (0,0,0) (0,2,0) (2,0,0)
        //     edge 0: (0,0,0)→(0,2,0) (was (0,0,0)→(2,0,0) before flip)
        //     But the edge map was built BEFORE flip, so edge (0,0,0)-(2,0,0)
        //     still has both triangles.
        //
        // The issue: when we flip tri1, the edge vertex order changes but the
        // edge hash is canonical (sorted vertices), so the hash is the same.
        // The effective direction comparison correctly accounts for the flip.
        //
        // After all edges processed, tri1 is flipped once.
        // Then centroid vote: component has tris [0,1].
        // Compute centroid, vote for each triangle's normal vs centroid ray.
        // Tri0 normal (effective = since not flipped) = +Z
        //   center = (0.67, 0.67, 0), centroid = (0.33, 0.33, 0)
        //   centroid ray = (0.33, 0.33, 0), dot(+Z, ...) = 0 → hard case
        // Actually all points are in the z=0 plane, so dot with +Z is always 0.
        // outward_votes will be... 0 since dot == 0.0 which is >= 0.0 → outward += 1
        // Same for tri1 (effective normal = -Z, dot = 0) → outward += 1
        // Both vote outward → no component flip.
        let flips = normalize_winding(&mut p);
        // After processing: tri1 was flipped once during BFS + 0 component flips
        // = 1 flip
        assert_eq!(flips, 1);
    }

    #[test]
    fn normalize_winding_chain_propagation() {
        // Three triangles A-B-C, each sharing an edge with next.
        // A and B share edge (0,0,0)-(4,0,0) with SAME direction → B inverted.
        // After B flipped, B and C share edge (0,0,0)-(4,0,4).
        // B's effective edge direction is (0,0,0)→(4,0,4), C's original
        // is also (0,0,0)→(4,0,4) → same → C inverted too.
        // Expected: 2 flips (B and C).
        let mut p = vec![
            0.0, 0.0, 0.0, 4.0, 0.0, 0.0, 0.0, 4.0, 0.0,  // A
            0.0, 0.0, 0.0, 4.0, 0.0, 0.0, 4.0, 0.0, 4.0,  // B (same dir on shared edge)
            0.0, 0.0, 0.0, 4.0, 0.0, 4.0, 4.0, 4.0, 4.0,  // C (same dir on shared edge)
        ];
        let flips = normalize_winding(&mut p);
        assert_eq!(flips, 2, "B and C should be flipped");
    }

    #[test]
    fn normalize_winding_degenerate_edge_skipped() {
        // Triangle with two identical vertices → degenerate edge skipped
        let mut p = vec![
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0,
        ];
        assert_eq!(normalize_winding(&mut p), 0);
    }

    #[test]
    fn normalize_winding_disconnected_components() {
        // Two separate pairs of triangles, not sharing any edge
        // Pair 1: (0,0,0)-(1,0,0)-(0,1,0) and inverted copy
        // Pair 2: (10,0,0)-(11,0,0)-(10,1,0) and inverted copy
        let mut p = vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0,  // P1A (correct)
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,  // P1B (inverted same dir)
            10.0, 0.0, 0.0, 11.0, 0.0, 0.0, 10.0, 1.0, 0.0, // P2A (correct)
            10.0, 0.0, 0.0, 11.0, 0.0, 0.0, 10.0, 0.0, 1.0, // P2B (inverted)
        ];
        let flips = normalize_winding(&mut p);
        assert_eq!(flips, 2, "Both inverted triangles flipped");
    }
}
