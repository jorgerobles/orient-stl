use std::collections::{HashMap, HashSet, VecDeque};

/// Minimum connected-component size for centroid-based outward-orientation voting.
pub const MIN_COMPONENT_VOTE: usize = 4;

/// Default vertex welding epsilon.
pub const DEFAULT_WELD_EPSILON: f32 = 1e-5;

/// Maximum number of boundary edges per hole to fill.
pub const DEFAULT_MAX_HOLE_EDGES: u32 = 512;

/// Flat-array compatibility layer for the geometry kernel.
/// These functions accept `&[f32]` position arrays (9 floats per triangle)
/// and return `Vec<f32>` — matching the interface of core/src/repair.rs.

/// Remove duplicate triangles from a flat position array.
/// Returns the number of triangles removed.
pub fn repair_mesh(positions: &mut Vec<f32>) -> u32 {
    let n = positions.len() / 9;
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
        tri.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                .then_with(|| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
        });

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

fn hash_tri(tri: &[(f32, f32, f32); 3]) -> u64 {
    let mut h = 14695981039346656037u64;
    for v in tri {
        for bits in [v.0.to_bits(), v.1.to_bits(), v.2.to_bits()] {
            for b in bits.to_le_bytes() {
                h ^= b as u64;
                h = h.wrapping_mul(1099511628211);
            }
        }
    }
    h
}

fn vertex_hash(x: f32, y: f32, z: f32) -> u64 {
    let mut h = 14695981039346656037u64;
    for bits in [x.to_bits(), y.to_bits(), z.to_bits()] {
        for b in bits.to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(1099511628211);
        }
    }
    h
}

fn edge_hash(ax: f32, ay: f32, az: f32, bx: f32, by: f32, bz: f32) -> u64 {
    let ha = vertex_hash(ax, ay, az);
    let hb = vertex_hash(bx, by, bz);
    // Sort hashes so edge_hash is symmetric (commutative) regardless of vertex order.
    let (a, b) = if ha <= hb { (ha, hb) } else { (hb, ha) };
    a ^ b.wrapping_mul(0x9e3779b97f4a7c15)
}

/// Fix face normals by propagating orientation from a seed triangle,
/// then voting per-component via centroid. Returns number of flipped triangles.
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
            let (ax, ay, az) = (positions[base + a_off], positions[base + a_off + 1], positions[base + a_off + 2]);
            let (bx, by, bz) = (positions[base + b_off], positions[base + b_off + 1], positions[base + b_off + 2]);
            if ax == bx && ay == by && az == bz {
                continue;
            }
            let key = edge_hash(ax, ay, az, bx, by, bz);
            edge_map.entry(key).or_default().push((i, e));
        }
    }

    let mut should_flip = vec![false; n];
    let mut visited = vec![false; n];
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
                let (ax, ay, az) = (positions[base + a_off], positions[base + a_off + 1], positions[base + a_off + 2]);
                let (bx, by, bz) = (positions[base + b_off], positions[base + b_off + 1], positions[base + b_off + 2]);
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

                    let (tri_sx, tri_sy, tri_sz, tri_ex, tri_ey, tri_ez) =
                        if should_flip[tri] { (bx, by, bz, ax, ay, az) } else { (ax, ay, az, bx, by, bz) };

                    let n_base = neighbor * 9;
                    let na_off = n_edge as usize * 3;
                    let nb_off = ((n_edge as usize + 1) % 3) * 3;
                    let (n_ax, n_ay, n_az) = (positions[n_base + na_off], positions[n_base + na_off + 1], positions[n_base + na_off + 2]);
                    let (n_bx, n_by, n_bz) = (positions[n_base + nb_off], positions[n_base + nb_off + 1], positions[n_base + nb_off + 2]);

                    let (n_sx, n_sy, n_sz, n_ex, n_ey, n_ez) = if should_flip[neighbor] {
                        (n_bx, n_by, n_bz, n_ax, n_ay, n_az)
                    } else {
                        (n_ax, n_ay, n_az, n_bx, n_by, n_bz)
                    };

                    let consistent = tri_sx == n_ex && tri_sy == n_ey && tri_sz == n_ez
                        && tri_ex == n_sx && tri_ey == n_sy && tri_ez == n_sz;

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

    // Per-component centroid voting
    const MIN_COMPONENT_VOTE: usize = 4;
    for comp in &components {
        if comp.len() < MIN_COMPONENT_VOTE {
            continue;
        }
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
        if verts == 0 { continue; }
        let (cx, cy, cz) = (cx / verts as f64, cy / verts as f64, cz / verts as f64);

        let mut outward_votes = 0i64;
        for &tri in comp {
            let base = tri * 9;
            let v1 = [positions[base], positions[base + 1], positions[base + 2]];
            let v2 = [positions[base + 3], positions[base + 4], positions[base + 5]];
            let v3 = [positions[base + 6], positions[base + 7], positions[base + 8]];
            let e1 = [v2[0]-v1[0], v2[1]-v1[1], v2[2]-v1[2]];
            let e2 = [v3[0]-v1[0], v3[1]-v1[1], v3[2]-v1[2]];
            let n = [e1[1]*e2[2]-e1[2]*e2[1], e1[2]*e2[0]-e1[0]*e2[2], e1[0]*e2[1]-e1[1]*e2[0]];
            let len_sq = n[0]*n[0] + n[1]*n[1] + n[2]*n[2];
            if len_sq <= f32::EPSILON { continue; }
            let tc = [(v1[0]+v2[0]+v3[0])/3.0, (v1[1]+v2[1]+v3[1])/3.0, (v1[2]+v2[2]+v3[2])/3.0];
            let d = [tc[0]-cx as f32, tc[1]-cy as f32, tc[2]-cz as f32];
            let eff_n = if should_flip[tri] { [-n[0],-n[1],-n[2]] } else { n };
            if eff_n[0]*d[0] + eff_n[1]*d[1] + eff_n[2]*d[2] >= 0.0 {
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

/// Weld nearby vertices within epsilon distance.
pub fn weld_vertices(positions: &mut Vec<f32>, epsilon: f32) -> u32 {
    let tri_count = positions.len() / 9;
    if tri_count == 0 || epsilon <= 0.0 {
        return 0;
    }

    let inv_eps = epsilon.recip();
    let mut grid: HashMap<(i64, i64, i64), [f32; 3]> = HashMap::new();
    let mut welded = 0u32;

    for tri in 0..tri_count {
        let base = tri * 9;
        for slot in 0..3 {
            let off = slot * 3;
            let v = [positions[base + off], positions[base + off + 1], positions[base + off + 2]];
            if !v[0].is_finite() || !v[1].is_finite() || !v[2].is_finite() {
                continue;
            }
            let key = ((v[0]*inv_eps).floor() as i64, (v[1]*inv_eps).floor() as i64, (v[2]*inv_eps).floor() as i64);

            let mut canonical: Option<[f32; 3]> = None;
            'cells: for dx in -1i64..=1 {
                for dy in -1i64..=1 {
                    for dz in -1i64..=1 {
                        let nk = (key.0+dx, key.1+dy, key.2+dz);
                        if let Some(&c) = grid.get(&nk) {
                            let ddx = v[0]-c[0]; let ddy = v[1]-c[1]; let ddz = v[2]-c[2];
                            if ddx*ddx + ddy*ddy + ddz*ddz <= epsilon*epsilon {
                                canonical = Some(c);
                                break 'cells;
                            }
                        }
                    }
                }
            }

            match canonical {
                Some(c) => {
                    if v[0] != c[0] || v[1] != c[1] || v[2] != c[2] {
                        positions[base + off] = c[0];
                        positions[base + off + 1] = c[1];
                        positions[base + off + 2] = c[2];
                        welded += 1;
                    }
                }
                None => {
                    grid.insert(key, v);
                }
            }
        }
    }
    welded
}

/// Count boundary edges (edges appearing in exactly 1 triangle).
pub fn count_boundary_edges(positions: &[f32]) -> u32 {
    let n = positions.len() / 9;
    if n == 0 { return 0; }

    let mut edge_count: HashMap<u64, u32> = HashMap::new();
    for i in 0..n {
        let base = i * 9;
        for e in 0..3u8 {
            let a_off = e as usize * 3;
            let b_off = ((e as usize + 1) % 3) * 3;
            let (ax, ay, az) = (positions[base+a_off], positions[base+a_off+1], positions[base+a_off+2]);
            let (bx, by, bz) = (positions[base+b_off], positions[base+b_off+1], positions[base+b_off+2]);
            if ax == bx && ay == by && az == bz { continue; }
            let key = edge_hash(ax, ay, az, bx, by, bz);
            *edge_count.entry(key).or_insert(0) += 1;
        }
    }
    edge_count.values().filter(|&&c| c == 1).count() as u32
}

/// Fill boundary holes. Returns number of triangles added.
pub fn fill_holes(positions: &mut Vec<f32>, max_edges: u32) -> u32 {
    let n = positions.len() / 9;
    if n == 0 { return 0; }

    let mut edge_map: HashMap<u64, Vec<(usize, u8)>> = HashMap::new();
    for i in 0..n {
        let base = i * 9;
        for e in 0..3u8 {
            let a_off = e as usize * 3;
            let b_off = ((e as usize + 1) % 3) * 3;
            let (ax, ay, az) = (positions[base+a_off], positions[base+a_off+1], positions[base+a_off+2]);
            let (bx, by, bz) = (positions[base+b_off], positions[base+b_off+1], positions[base+b_off+2]);
            if ax == bx && ay == by && az == bz { continue; }
            let key = edge_hash(ax, ay, az, bx, by, bz);
            edge_map.entry(key).or_default().push((i, e));
        }
    }

    struct BEdge { sx: f32, sy: f32, sz: f32, ex: f32, ey: f32, ez: f32 }
    let mut boundary: Vec<BEdge> = Vec::new();
    for (_key, tris) in &edge_map {
        if tris.len() != 1 { continue; }
        let (ti, slot) = tris[0];
        let base = ti * 9;
        let a_off = slot as usize * 3;
        let b_off = ((slot as usize + 1) % 3) * 3;
        let (ax, ay, az) = (positions[base+a_off], positions[base+a_off+1], positions[base+a_off+2]);
        let (bx, by, bz) = (positions[base+b_off], positions[base+b_off+1], positions[base+b_off+2]);
        if ax == bx && ay == by && az == bz { continue; }
        boundary.push(BEdge { sx: ax, sy: ay, sz: az, ex: bx, ey: by, ez: bz });
    }

    if boundary.is_empty() { return 0; }

    let mut next_map: HashMap<u64, Vec<[f32; 3]>> = HashMap::new();
    for be in &boundary {
        let h = vertex_hash(be.sx, be.sy, be.sz);
        next_map.entry(h).or_default().push([be.ex, be.ey, be.ez]);
    }

    let mut visited_edges: HashSet<(u64, u64)> = HashSet::new();
    let mut loops: Vec<Vec<[f32; 3]>> = Vec::new();
    let edge_key = |a: &[f32; 3], b: &[f32; 3]| -> (u64, u64) {
        (vertex_hash(a[0], a[1], a[2]), vertex_hash(b[0], b[1], b[2]))
    };

    for be in &boundary {
        let start = [be.sx, be.sy, be.sz];
        let end = [be.ex, be.ey, be.ez];
        let ek = edge_key(&start, &end);
        if visited_edges.contains(&ek) { continue; }

        let mut loop_pts: Vec<[f32; 3]> = vec![start, end];
        visited_edges.insert(ek);
        let mut current = end;

        loop {
            let ch = vertex_hash(current[0], current[1], current[2]);
            let candidates = match next_map.get(&ch) { Some(v) => v, None => break };
            let mut found = None;
            for next_pt in candidates {
                let ek2 = edge_key(&current, next_pt);
                if !visited_edges.contains(&ek2) {
                    found = Some(*next_pt);
                    visited_edges.insert(ek2);
                    break;
                }
            }
            match found {
                Some(next_pt) => {
                    let sh = vertex_hash(start[0], start[1], start[2]);
                    let nh = vertex_hash(next_pt[0], next_pt[1], next_pt[2]);
                    if nh == sh { break; }
                    loop_pts.push(next_pt);
                    current = next_pt;
                }
                None => break,
            }
            if loop_pts.len() > max_edges as usize + 4 { break; }
        }

        if loop_pts.len() >= 3 { loops.push(loop_pts); }
    }

    let mut added = 0u32;
    for loop_pts in &loops {
        if loop_pts.len() > max_edges as usize { continue; }
        let tris = ear_clip_loop(loop_pts);
        for tri in &tris {
            for v in tri {
                positions.push(v[0]); positions.push(v[1]); positions.push(v[2]);
            }
        }
        added += tris.len() as u32;
    }
    added
}

fn ear_clip_loop(loop_pts: &[[f32; 3]]) -> Vec<[[f32; 3]; 3]> {
    if loop_pts.len() < 3 { return Vec::new(); }
    let n = loop_pts.len();

    // Compute centroid
    let (mut cx, mut cy, mut cz) = (0.0f64, 0.0f64, 0.0f64);
    for p in loop_pts {
        cx += p[0] as f64;
        cy += p[1] as f64;
        cz += p[2] as f64;
    }
    cx /= n as f64;
    cy /= n as f64;
    cz /= n as f64;

    // Compute best-fit plane normal via cross-product accumulation
    let mut nx = 0.0f64;
    let mut ny = 0.0f64;
    let mut nz = 0.0f64;
    for i in 0..n {
        let a = &loop_pts[i];
        let b = &loop_pts[(i + 1) % n];
        let e1 = [a[0] as f64 - cx, a[1] as f64 - cy, a[2] as f64 - cz];
        let e2 = [b[0] as f64 - cx, b[1] as f64 - cy, b[2] as f64 - cz];
        nx += e1[1] * e2[2] - e1[2] * e2[1];
        ny += e1[2] * e2[0] - e1[0] * e2[2];
        nz += e1[0] * e2[1] - e1[1] * e2[0];
    }
    let len = (nx * nx + ny * ny + nz * nz).sqrt();
    if len < 1e-20 {
        // Degenerate — fallback to simple fan
        let mut tris = Vec::new();
        for i in 1..n - 1 {
            tris.push([loop_pts[0], loop_pts[i], loop_pts[i + 1]]);
        }
        return tris;
    }
    nx /= len; ny /= len; nz /= len;

    // Build two orthonormal axes in the plane
    let (ux, uy, uz) = if nx.abs() > 0.9 {
        (0.0f64, 1.0, 0.0)
    } else {
        (1.0f64, 0.0, 0.0)
    };
    // t = u - (u·n)n, normalized
    let dot = ux * nx + uy * ny + uz * nz;
    let (mut tx, mut ty, mut tz) = (ux - dot * nx, uy - dot * ny, uz - dot * nz);
    let tl = (tx * tx + ty * ty + tz * tz).sqrt();
    tx /= tl; ty /= tl; tz /= tl;
    // b = n × t
    let (bx, by, bz) = (ny * tz - nz * ty, nz * tx - nx * tz, nx * ty - ny * tx);

    // Project loop onto 2D plane
    let proj: Vec<[f64; 2]> = loop_pts.iter().map(|p| {
        let dx = p[0] as f64 - cx;
        let dy = p[1] as f64 - cy;
        let dz = p[2] as f64 - cz;
        [dx * tx + dy * ty + dz * tz, dx * bx + dy * by + dz * bz]
    }).collect();

    // 2D ear clipping on projected points
    let mut verts_2d: Vec<[f64; 2]> = proj;
    let mut verts_3d: Vec<[f32; 3]> = loop_pts.to_vec();
    let mut tris = Vec::new();

    while verts_2d.len() > 3 {
        let m = verts_2d.len();
        let mut ear_found = false;
        for i in 0..m {
            let prev_i = (i + m - 1) % m;
            let next_i = (i + 1) % m;
            let prev = verts_2d[prev_i];
            let curr = verts_2d[i];
            let next = verts_2d[next_i];

            // Cross product in 2D (z-component)
            let cross = (curr[0] - prev[0]) * (next[1] - curr[1])
                       - (curr[1] - prev[1]) * (next[0] - curr[0]);
            if cross.abs() < 1e-20 { continue; }

            let mut has_interior = false;
            for (j, &v) in verts_2d.iter().enumerate() {
                if j == prev_i || j == i || j == next_i { continue; }
                // Barycentric test in 2D
                let abx = curr[0] - prev[0]; let aby = curr[1] - prev[1];
                let acx = next[0] - prev[0]; let acy = next[1] - prev[1];
                let apx = v[0] - prev[0];    let apy = v[1] - prev[1];
                let d00 = abx * abx + aby * aby;
                let d01 = abx * acx + aby * acy;
                let d11 = acx * acx + acy * acy;
                let d20 = apx * abx + apy * aby;
                let d21 = apx * acx + apy * acy;
                let denom = d00 * d11 - d01 * d01;
                if denom.abs() < 1e-20 { continue; }
                let u = (d11 * d20 - d01 * d21) / denom;
                let v = (d00 * d21 - d01 * d20) / denom;
                if u > 0.0 && v > 0.0 && u + v < 1.0 {
                    has_interior = true;
                    break;
                }
            }

            if !has_interior {
                tris.push([verts_3d[prev_i], verts_3d[i], verts_3d[next_i]]);
                verts_2d.remove(i);
                verts_3d.remove(i);
                ear_found = true;
                break;
            }
        }
        if !ear_found { break; }
    }

    if verts_2d.len() == 3 {
        tris.push([verts_3d[0], verts_3d[1], verts_3d[2]]);
    }
    tris
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
        let mut p = vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        assert_eq!(repair_mesh(&mut p), 0);
        assert_eq!(p.len(), 18);
    }

    // ─── normalize_winding tests ──

    #[test]
    fn normalize_winding_empty() {
        let mut p: Vec<f32> = Vec::new();
        assert_eq!(normalize_winding(&mut p), 0);
    }

    #[test]
    fn normalize_winding_single_triangle() {
        let mut p = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        assert_eq!(normalize_winding(&mut p), 0);
    }

    #[test]
    fn normalize_winding_two_triangles_consistent() {
        let mut p = vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
            1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let flips = normalize_winding(&mut p);
        assert_eq!(flips, 0);
    }

    #[test]
    fn normalize_winding_two_triangles_inverted() {
        let mut p = vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let flips = normalize_winding(&mut p);
        assert_eq!(flips, 1, "Tri B should be flipped");
        assert!((p[9 + 3] - 0.0).abs() < 1e-6);
        assert!((p[9 + 4] - 0.0).abs() < 1e-6);
        assert!((p[9 + 5] - 1.0).abs() < 1e-6);
        assert!((p[9 + 6] - 1.0).abs() < 1e-6);
        assert!((p[9 + 7] - 0.0).abs() < 1e-6);
        assert!((p[9 + 8] - 0.0).abs() < 1e-6);
    }

    #[test]
    fn normalize_winding_thin_shell() {
        let mut p = vec![
            0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 2.0, 0.0,
            0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 2.0, 0.0,
        ];
        let flips = normalize_winding(&mut p);
        assert_eq!(flips, 1);
    }

    #[test]
    fn normalize_winding_chain_propagation() {
        let mut p = vec![
            0.0, 0.0, 0.0, 4.0, 0.0, 0.0, 0.0, 4.0, 0.0,
            0.0, 0.0, 0.0, 4.0, 0.0, 0.0, 4.0, 0.0, 4.0,
            0.0, 0.0, 0.0, 4.0, 0.0, 4.0, 4.0, 4.0, 4.0,
        ];
        let flips = normalize_winding(&mut p);
        assert_eq!(flips, 2, "B and C should be flipped");
    }

    #[test]
    fn normalize_winding_degenerate_edge_skipped() {
        let mut p = vec![
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0,
        ];
        assert_eq!(normalize_winding(&mut p), 0);
    }

    #[test]
    fn normalize_winding_disconnected_components() {
        let mut p = vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            10.0, 0.0, 0.0, 11.0, 0.0, 0.0, 10.0, 1.0, 0.0,
            10.0, 0.0, 0.0, 11.0, 0.0, 0.0, 10.0, 0.0, 1.0,
        ];
        let flips = normalize_winding(&mut p);
        assert_eq!(flips, 2, "Both inverted triangles flipped");
    }

    // ─── fill_holes tests ─────────────────────────────────

    fn hole_ring(hole_verts: &[[f32; 3]], center: &[f32; 3]) -> Vec<f32> {
        let n = hole_verts.len();
        let mut p = Vec::with_capacity(n * 9);
        for i in 0..n {
            let j = (i + 1) % n;
            p.push(hole_verts[i][0]); p.push(hole_verts[i][1]); p.push(hole_verts[i][2]);
            p.push(hole_verts[j][0]); p.push(hole_verts[j][1]); p.push(hole_verts[j][2]);
            p.push(center[0]); p.push(center[1]); p.push(center[2]);
        }
        p
    }

    #[test]
    fn fill_holes_empty_mesh() {
        let mut p = Vec::new();
        assert_eq!(fill_holes(&mut p, 64), 0);
    }

    #[test]
    fn fill_holes_square_hole() {
        let verts = [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [2.0, 2.0, 0.0], [0.0, 2.0, 0.0]];
        let mut p = hole_ring(&verts, &[10.0, 10.0, 10.0]);
        let added = fill_holes(&mut p, 64);
        assert_eq!(added, 2, "square hole should close with 2 triangles");
        assert_eq!(p.len(), 54, "4 + 2 = 6 triangles → 54 floats");
    }

    #[test]
    fn fill_holes_triangle_hole() {
        let verts = [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [1.0, 2.0, 0.0]];
        let mut p = hole_ring(&verts, &[10.0, 10.0, 10.0]);
        let added = fill_holes(&mut p, 64);
        assert_eq!(added, 1);
        assert_eq!(p.len(), 36, "3 + 1 = 4 → 36 floats");
    }

    #[test]
    fn fill_holes_hexagon_hole() {
        let verts = [
            [1.5, 0.0, 0.0], [3.0, 0.866, 0.0], [3.0, 2.598, 0.0],
            [1.5, 3.464, 0.0], [0.0, 2.598, 0.0], [0.0, 0.866, 0.0],
        ];
        let mut p = hole_ring(&verts, &[10.0, 10.0, 10.0]);
        let added = fill_holes(&mut p, 64);
        assert_eq!(added, 4, "hexagon → 4 triangles");
        assert_eq!(p.len(), 90, "6 + 4 = 10 → 90 floats");
    }

    #[test]
    fn fill_holes_skips_large_hole() {
        let verts = [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [2.0, 2.0, 0.0], [0.0, 2.0, 0.0]];
        let mut p = hole_ring(&verts, &[10.0, 10.0, 10.0]);
        let added = fill_holes(&mut p, 2);
        assert_eq!(added, 0);
    }

    #[test]
    fn fill_holes_no_boundary_no_fill() {
        let mut p = vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
            1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0,
        ];
        let added = fill_holes(&mut p, 64);
        assert_eq!(added, 2);
    }

    // ─── weld_vertices tests ────────────────────────────

    #[test]
    fn weld_empty_mesh() {
        let mut p = Vec::new();
        assert_eq!(weld_vertices(&mut p, 1e-5), 0);
    }

    #[test]
    fn weld_zero_epsilon_no_op() {
        let mut p = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        assert_eq!(weld_vertices(&mut p, 0.0), 0);
        assert_eq!(p.len(), 9);
    }

    #[test]
    fn weld_exact_vertices_no_change() {
        let orig = vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let mut p = orig.clone();
        let welded = weld_vertices(&mut p, 1e-5);
        // Exact duplicates match canonical but coordinates are identical → 0 actual changes
        assert_eq!(welded, 0);
        assert_eq!(p, orig);
    }

    #[test]
    fn weld_nearby_vertices_snapped() {
        let mut p = vec![
            0.0, 0.0, 0.0,   1.0, 0.0, 0.0,   0.0, 1.0, 0.0,
            1e-6, 0.0, 0.0,  1.0, 1.0, 0.0,   0.0, 0.0, 1.0,
        ];
        let welded = weld_vertices(&mut p, 1e-5);
        assert_eq!(welded, 1, "near-origin vertex should snap to origin");
        assert_eq!(p[9], 0.0, "v3.x should be 0 after weld");
        assert_eq!(p[10], 0.0, "v3.y should be 0 after weld");
        assert_eq!(p[11], 0.0, "v3.z should be 0 after weld");
    }

    #[test]
    fn weld_far_vertices_unchanged() {
        let orig = vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
            2.0, 0.0, 0.0, 3.0, 0.0, 0.0, 2.0, 1.0, 0.0,
        ];
        let mut p = orig.clone();
        let welded = weld_vertices(&mut p, 1e-5);
        assert_eq!(welded, 0);
        assert_eq!(p, orig);
    }

    #[test]
    fn weld_merges_boundary_edges() {
        let mut p = vec![
            0.0, 0.0, 0.0,  2.0, 0.0, 0.0,  2.0, 2.0, 0.0,
            0.0, 0.0, 0.0,  2.0, 2.0, 0.0,  0.0, 2.0, 0.0,
            2.0+1e-6, 0.0, 0.0,  4.0, 0.0, 0.0,  4.0, 2.0, 0.0,
            2.0+1e-6, 0.0, 0.0,  4.0, 2.0, 0.0,  2.0+1e-6, 2.0, 0.0,
        ];
        let before = count_boundary_edges_test(&p);
        let welded = weld_vertices(&mut p, 1e-5);
        assert!(welded > 0, "should weld shared-edge vertices");
        repair_mesh(&mut p);
        let after = count_boundary_edges_test(&p);
        assert!(after < before, "welding should reduce boundary edges: {before}→{after}");
    }

    fn count_boundary_edges_test(positions: &[f32]) -> usize {
        use std::collections::HashMap;
        let n = positions.len() / 9;
        let mut edge_map: HashMap<u64, Vec<(usize, u8)>> = HashMap::new();
        for i in 0..n {
            let base = i * 9;
            for e in 0..3u8 {
                let a_off = e as usize * 3;
                let b_off = ((e as usize + 1) % 3) * 3;
                let (ax, ay, az) = (positions[base + a_off], positions[base + a_off + 1], positions[base + a_off + 2]);
                let (bx, by, bz) = (positions[base + b_off], positions[base + b_off + 1], positions[base + b_off + 2]);
                if ax == bx && ay == by && az == bz { continue; }
                let key = edge_hash(ax, ay, az, bx, by, bz);
                edge_map.entry(key).or_default().push((i, e));
            }
        }
        edge_map.values().filter(|v| v.len() == 1).count()
    }

    #[test]
    fn ear_clip_convex_pentagon() {
        let pentagon: Vec<[f32; 3]> = vec![
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [3.0, 1.0, 0.0],
            [1.0, 3.0, 0.0],
            [-1.0, 1.0, 0.0],
        ];
        let tris = ear_clip_loop(&pentagon);
        assert_eq!(tris.len(), 3, "pentagon → 3 triangles");
        for (i, tri) in tris.iter().enumerate() {
            assert!((tri[0][2]).abs() < 1e-5, "tri {i} v0 z");
            assert!((tri[1][2]).abs() < 1e-5, "tri {i} v1 z");
            assert!((tri[2][2]).abs() < 1e-5, "tri {i} v2 z");
            let e1x = tri[1][0] - tri[0][0];
            let e1y = tri[1][1] - tri[0][1];
            let e2x = tri[2][0] - tri[0][0];
            let e2y = tri[2][1] - tri[0][1];
            let area = (e1x * e2y - e1y * e2x).abs() * 0.5;
            assert!(area > 0.01, "tri {i} area={area}");
        }
    }
}
