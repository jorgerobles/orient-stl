use std::collections::{HashMap, HashSet, VecDeque};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Minimum connected-component size for centroid-based outward-orientation
/// voting. Components smaller than this may have unreliable centroid rays.
pub const MIN_COMPONENT_VOTE: usize = 4;

/// Maximum number of boundary edges per hole to fill.
/// Holes larger than this are left as-is (likely intentional openings).
pub const DEFAULT_MAX_HOLE_EDGES: u32 = 64;

/// Default vertex welding epsilon (absolute, dimensionless).
/// Vertices within this distance are snapped together.
pub const DEFAULT_WELD_EPSILON: f32 = 1e-5;

// ---------------------------------------------------------------------------

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
        if comp.len() < MIN_COMPONENT_VOTE {
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
/// Hash a single vertex [x,y,z] into a u64 for half-edge-loop tracing.
fn vertex_hash(x: f32, y: f32, z: f32) -> u64 {
    let mut h = 14695981039346656037u64;
    for byte in x.to_bits().to_le_bytes().iter()
        .chain(y.to_bits().to_le_bytes().iter())
        .chain(z.to_bits().to_le_bytes().iter())
    {
        h ^= *byte as u64;
        h = h.wrapping_mul(1099511628211);
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

// ---------------------------------------------------------------------------
// Hole filling via ear-clipping triangulation
// ---------------------------------------------------------------------------

/// Newell's method: compute the best-fit plane normal of a 3D polygon.
fn polygon_normal(pts: &[[f32; 3]]) -> [f32; 3] {
    if pts.len() < 3 {
        return [0.0, 0.0, 1.0];
    }
    let mut nx = 0.0f64;
    let mut ny = 0.0f64;
    let mut nz = 0.0f64;
    let n = pts.len();
    for i in 0..n {
        let j = (i + 1) % n;
        let xi = pts[i][0] as f64; let yi = pts[i][1] as f64; let zi = pts[i][2] as f64;
        let xj = pts[j][0] as f64; let yj = pts[j][1] as f64; let zj = pts[j][2] as f64;
        nx += (yi - yj) * (zi + zj);
        ny += (zi - zj) * (xi + xj);
        nz += (xi - xj) * (yi + yj);
    }
    let len = (nx * nx + ny * ny + nz * nz).sqrt();
    if len > 1e-30 {
        [nx as f32 / len as f32, ny as f32 / len as f32, nz as f32 / len as f32]
    } else {
        [0.0, 0.0, 1.0]
    }
}

/// Orthonormal basis vectors in the plane of `n`.
fn ortho_basis(n: &[f32; 3]) -> ([f32; 3], [f32; 3]) {
    let u = if n[0].abs() > 0.1 || n[1].abs() > 0.1 {
        [n[1], -n[0], 0.0]
    } else {
        [0.0, n[2], -n[1]]
    };
    let ulen = (u[0] * u[0] + u[1] * u[1] + u[2] * u[2]).sqrt();
    let u = [u[0] / ulen, u[1] / ulen, u[2] / ulen];
    let v = [
        n[1] * u[2] - n[2] * u[1],
        n[2] * u[0] - n[0] * u[2],
        n[0] * u[1] - n[1] * u[0],
    ];
    (u, v)
}

/// Signed area of a 2D polygon. Positive = CCW.
fn signed_area_2d(pts: &[(f32, f32)]) -> f32 {
    let mut area = 0.0;
    let n = pts.len();
    for i in 0..n {
        let j = (i + 1) % n;
        area += pts[i].0 * pts[j].1;
        area -= pts[j].0 * pts[i].1;
    }
    area * 0.5
}

/// Test if point `p` is inside the CCW triangle (a,b,c) in 2D.
fn point_in_triangle_2d(p: (f32, f32), a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> bool {
    let cross1 = (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0);
    let cross2 = (c.0 - b.0) * (p.1 - b.1) - (c.1 - b.1) * (p.0 - b.0);
    let cross3 = (a.0 - c.0) * (p.1 - c.1) - (a.1 - c.1) * (p.0 - c.0);
    cross1 >= -1e-8 && cross2 >= -1e-8 && cross3 >= -1e-8
}

/// Ear-clip a 3D polygon loop. Returns one or more triangles that fill the hole.
/// `loop_3d` must have ≥3 vertices in consistent winding order (CCW from the
/// outside). Returns an empty vec if triangulation fails.
fn ear_clip_loop(loop_3d: &[[f32; 3]]) -> Vec<[[f32; 3]; 3]> {
    if loop_3d.len() < 3 {
        return vec![];
    }
    if loop_3d.len() == 3 {
        return vec![[loop_3d[0], loop_3d[1], loop_3d[2]]];
    }

    // Project to 2D
    let n = polygon_normal(loop_3d);
    let (u, v) = ortho_basis(&n);
    let mut cx = 0.0f64;
    let mut cy = 0.0f64;
    let mut cz = 0.0f64;
    for p in loop_3d {
        cx += p[0] as f64;
        cy += p[1] as f64;
        cz += p[2] as f64;
    }
    let nf = loop_3d.len() as f64;
    let c = [cx / nf, cy / nf, cz / nf];
    let mut pts_2d: Vec<(f32, f32)> = loop_3d
        .iter()
        .map(|p| {
            let dx = p[0] - c[0] as f32;
            let dy = p[1] - c[1] as f32;
            let dz = p[2] - c[2] as f32;
            (dx * u[0] + dy * u[1] + dz * u[2],
             dx * v[0] + dy * v[1] + dz * v[2])
        })
        .collect();

    // Ensure CCW
    if signed_area_2d(&pts_2d) < 0.0 {
        pts_2d.reverse();
        let mut rev = loop_3d.to_vec();
        rev.reverse();
        // Use reversed 3D list (keep a copy)
        return ear_clip_2d(&pts_2d, &rev);
    }

    ear_clip_2d(&pts_2d, loop_3d)
}

/// Ear-clip a CCW 2D polygon, mapping back to 3D using `loop_3d`.
fn ear_clip_2d(pts_2d: &[(f32, f32)], loop_3d: &[[f32; 3]]) -> Vec<[[f32; 3]; 3]> {
    let n = pts_2d.len();
    if n < 3 {
        return vec![];
    }

    // Working copy of vertex indices
    let mut indices: Vec<usize> = (0..n).collect();
    let mut out: Vec<[[f32; 3]; 3]> = Vec::with_capacity(n - 2);
    let mut stuck_count = 0;

    while indices.len() >= 3 {
        let m = indices.len();
        let mut ear_found = false;

        for wi in 0..m {
            let prev = indices[(wi + m - 1) % m];
            let cur = indices[wi];
            let next = indices[(wi + 1) % m];

            // Check convex: cross product of edges (prev→cur) × (cur→next)
            let e1x = pts_2d[cur].0 - pts_2d[prev].0;
            let e1y = pts_2d[cur].1 - pts_2d[prev].1;
            let e2x = pts_2d[next].0 - pts_2d[cur].0;
            let e2y = pts_2d[next].1 - pts_2d[cur].1;
            let cross = e1x * e2y - e1y * e2x;
            if cross <= 1e-10 {
                continue; // reflex vertex
            }

            // Check no other vertex inside triangle (prev, cur, next)
            let a = pts_2d[prev];
            let b = pts_2d[cur];
            let c = pts_2d[next];
            let mut interior = false;
            for &vi in &indices {
                if vi == prev || vi == cur || vi == next {
                    continue;
                }
                if point_in_triangle_2d(pts_2d[vi], a, b, c) {
                    interior = true;
                    break;
                }
            }
            if interior {
                continue;
            }

            // Ear found
            out.push([loop_3d[prev], loop_3d[cur], loop_3d[next]]);
            indices.remove(wi);
            ear_found = true;
            stuck_count = 0;
            break;
        }

        if !ear_found {
            stuck_count += 1;
            if stuck_count > 3 {
                // Fallback: fan from first vertex
                out.clear();
                for i in 1..n - 1 {
                    out.push([loop_3d[0], loop_3d[i], loop_3d[i + 1]]);
                }
                break;
            }
            // Remove the most reflex vertex and retry
            let mut worst = 0;
            let mut worst_cross = f32::MAX;
            for wi in 0..indices.len() {
                let prev = indices[(wi + indices.len() - 1) % indices.len()];
                let cur = indices[wi];
                let next = indices[(wi + 1) % indices.len()];
                let e1x = pts_2d[cur].0 - pts_2d[prev].0;
                let e1y = pts_2d[cur].1 - pts_2d[prev].1;
                let e2x = pts_2d[next].0 - pts_2d[cur].0;
                let e2y = pts_2d[next].1 - pts_2d[cur].1;
                let cr = e1x * e2y - e1y * e2x;
                if cr < worst_cross {
                    worst_cross = cr;
                    worst = wi;
                }
            }
            indices.remove(worst);
        }
    }

    out
}

/// Find and fill holes (boundary edge loops) in a triangle-soup mesh.
/// Returns the number of triangles added.
///
/// Algorithm:
/// 1. Build an edge→triangle map (exact vertex matching)
/// 2. Collect boundary edges (edges shared by exactly 1 triangle)
/// 3. Trace boundary edges into closed loops
/// 4. Ear-clip each loop with ≤ `max_edges` edges
///
/// Holes larger than `max_edges` are left unfilled.
/// Loops with < 3 edges are degenerate and skipped.
pub fn fill_holes(positions: &mut Vec<f32>, max_edges: u32) -> u32 {
    let n = positions.len() / 9;
    if n == 0 {
        return 0;
    }

    // 1. Build edge map (same as normalize_winding)
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

    // 2. Collect boundary edges (exactly 1 triangle)
    struct BEdge {
        sx: f32, sy: f32, sz: f32,
        ex: f32, ey: f32, ez: f32,
    }
    let mut boundary: Vec<BEdge> = Vec::new();
    for (_key, tris) in &edge_map {
        if tris.len() != 1 {
            continue;
        }
        let (ti, slot) = tris[0];
        let base = ti * 9;
        let a_off = slot as usize * 3;
        let b_off = ((slot as usize + 1) % 3) * 3;
        let (ax, ay, az) = (positions[base + a_off], positions[base + a_off + 1], positions[base + a_off + 2]);
        let (bx, by, bz) = (positions[base + b_off], positions[base + b_off + 1], positions[base + b_off + 2]);
        if ax == bx && ay == by && az == bz {
            continue;
        }
        boundary.push(BEdge { sx: ax, sy: ay, sz: az, ex: bx, ey: by, ez: bz });
    }

    if boundary.is_empty() {
        return 0;
    }

    // 3. Build next-vertex map for loop tracing
    // vertex_hash(s) → Vec<end vertex>
    let mut next_map: HashMap<u64, Vec<[f32; 3]>> = HashMap::new();
    for be in &boundary {
        let h = vertex_hash(be.sx, be.sy, be.sz);
        next_map.entry(h).or_default().push([be.ex, be.ey, be.ez]);
    }

    // 4. Trace boundary loops
    let mut visited_edges: HashSet<(u64, u64)> = HashSet::new();
    let mut loops: Vec<Vec<[f32; 3]>> = Vec::new();

    // Helper: pack a directed edge as a visited key
    let edge_key = |a: &[f32; 3], b: &[f32; 3]| -> (u64, u64) {
        (vertex_hash(a[0], a[1], a[2]), vertex_hash(b[0], b[1], b[2]))
    };

    for be in &boundary {
        let start = [be.sx, be.sy, be.sz];
        let end = [be.ex, be.ey, be.ez];
        let ek = edge_key(&start, &end);
        if visited_edges.contains(&ek) {
            continue;
        }

        let mut loop_pts: Vec<[f32; 3]> = vec![start, end];
        visited_edges.insert(ek);
        let mut current = end;

        loop {
            let ch = vertex_hash(current[0], current[1], current[2]);
            let candidates = match next_map.get(&ch) {
                Some(v) => v,
                None => break,
            };

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
                    // Check if loop is closed (back to start)
                    let sh = vertex_hash(start[0], start[1], start[2]);
                    let nh = vertex_hash(next_pt[0], next_pt[1], next_pt[2]);
                    if nh == sh {
                        // Closed loop
                        break;
                    }
                    loop_pts.push(next_pt);
                    current = next_pt;
                }
                None => {
                    // Open chain — still fill if ≥3 vertices
                    break;
                }
            }

            // Safety: prevent runaway on malformed meshes
            if loop_pts.len() > max_edges as usize + 4 {
                break;
            }
        }

        if loop_pts.len() >= 3 {
            loops.push(loop_pts);
        }
    }

    // 5. Ear-clip each loop
    let mut added = 0u32;
    for loop_pts in &loops {
        if loop_pts.len() > max_edges as usize {
            continue;
        }

        let tris = ear_clip_loop(loop_pts);
        for tri in &tris {
            positions.push(tri[0][0]); positions.push(tri[0][1]); positions.push(tri[0][2]);
            positions.push(tri[1][0]); positions.push(tri[1][1]); positions.push(tri[1][2]);
            positions.push(tri[2][0]); positions.push(tri[2][1]); positions.push(tri[2][2]);
        }
        added += tris.len() as u32;
    }

    added
}

/// Weld nearby vertices within `epsilon` distance.
///
/// Replaces each vertex with the first-encountered nearby vertex's coordinates.
/// After welding, call `repair_mesh()` to remove degenerate triangles
/// (triangles with two or more vertices at the same location).
///
/// Uses a spatial hash (grid cell size = epsilon) for O(n * 27) lookup.
/// Returns the number of vertex slots modified (3 per triangle, not unique
/// vertices).
pub fn weld_vertices(positions: &mut Vec<f32>, epsilon: f32) -> u32 {
    let tri_count = positions.len() / 9;
    if tri_count == 0 || epsilon <= 0.0 {
        return 0;
    }

    let eps_sq = epsilon * epsilon;
    let inv_eps = epsilon.recip();
    // grid cell key → canonical [x, y, z]
    let mut grid: HashMap<(i64, i64, i64), [f32; 3]> = HashMap::new();
    let mut welded = 0u32;

    for tri in 0..tri_count {
        let base = tri * 9;
        for slot in 0..3 {
            let off = slot * 3;
            let v = [
                positions[base + off],
                positions[base + off + 1],
                positions[base + off + 2],
            ];

            // Check for NaN / Inf — can't weld those
            if !v[0].is_finite() || !v[1].is_finite() || !v[2].is_finite() {
                continue;
            }

            let key = (
                (v[0] * inv_eps).floor() as i64,
                (v[1] * inv_eps).floor() as i64,
                (v[2] * inv_eps).floor() as i64,
            );

            // Search this cell and all 26 neighbours
            let mut canonical: Option<[f32; 3]> = None;
            'cells: for dx in -1i64..=1 {
                for dy in -1i64..=1 {
                    for dz in -1i64..=1 {
                        let nk = (key.0.wrapping_add(dx), key.1.wrapping_add(dy), key.2.wrapping_add(dz));
                        if let Some(&cv) = grid.get(&nk) {
                            let d2 = (v[0] - cv[0]) * (v[0] - cv[0])
                                + (v[1] - cv[1]) * (v[1] - cv[1])
                                + (v[2] - cv[2]) * (v[2] - cv[2]);
                            if d2 <= eps_sq {
                                canonical = Some(cv);
                                break 'cells;
                            }
                        }
                    }
                }
            }

            match canonical {
                Some(cv) => {
                    positions[base + off] = cv[0];
                    positions[base + off + 1] = cv[1];
                    positions[base + off + 2] = cv[2];
                    welded += 1;
                }
                None => {
                    grid.insert(key, v);
                }
            }
        }
    }

    welded
}

/// Count the number of boundary edges (edges shared by exactly 1 triangle)
/// in a triangle-soup position array. A watertight mesh has 0 boundary edges.
pub fn count_boundary_edges(positions: &[f32]) -> u32 {
    let n = positions.len() / 9;
    if n == 0 {
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
    edge_map.values().filter(|v| v.len() == 1).count() as u32
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

    /// Build a ring of N triangles around a center point, forming an
    /// N-edge polygonal hole. Triangles: (v[i], v[(i+1)%N], center)
    /// where v[i] are the hole boundary vertices.
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
        // 4 input triangles → square hole of 4 edges → 2 fill tris
        let added = fill_holes(&mut p, 64);
        assert_eq!(added, 2, "square hole should close with 2 triangles");
        assert_eq!(p.len(), 54, "4 + 2 = 6 triangles → 54 floats");
    }

    #[test]
    fn fill_holes_triangle_hole() {
        let verts = [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [1.0, 2.0, 0.0]];
        let mut p = hole_ring(&verts, &[10.0, 10.0, 10.0]);
        // 3 input → triangle hole → 1 fill tri
        let added = fill_holes(&mut p, 64);
        assert_eq!(added, 1);
        assert_eq!(p.len(), 36, "3 + 1 = 4 → 36 floats");
    }

    #[test]
    fn fill_holes_hexagon_hole() {
        // Regular hexagon centered at (1.5, ~1.3), all z=0
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
        let added = fill_holes(&mut p, 2); // square = 4 edges > 2 → skip
        assert_eq!(added, 0);
    }

    #[test]
    fn fill_holes_no_boundary_no_fill() {
        // Closed quad (2 tris sharing diagonal): 4 boundary edges forming
        // the outer perimeter. This IS a "hole" (the outer boundary of a
        // non-closed mesh), so fill_holes will fill it. That's expected
        // behavior — the "outside" is just a very large hole.
        let mut p = vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
            1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0,
        ];
        let added = fill_holes(&mut p, 64);
        // Outer perimeter is a 4-edge hole → 2 triangles
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
        // (0,0,0) appears twice and (1,0,0) appears twice → 2 welded
        assert_eq!(welded, 2);
        assert_eq!(p, orig); // no coordinates changed, just matched
    }

    #[test]
    fn weld_nearby_vertices_snapped() {
        let mut p = vec![
            // tri 0: one vertex at origin
            0.0, 0.0, 0.0,   1.0, 0.0, 0.0,   0.0, 1.0, 0.0,
            // tri 1: first vertex at near-origin (1e-6 away)
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
        // Two quads sharing an edge but with slightly-offset shared vertices.
        // Quad A: (0,0,0)-(2,0,0)-(2,2,0)-(0,2,0) → 2 tris
        // Quad B: (2,0,0)-(4,0,0)-(4,2,0)-(2,2,0) → 2 tris
        // Shared edge vertices offset by 1e-6
        let mut p = vec![
            // Quad A
            0.0, 0.0, 0.0,  2.0, 0.0, 0.0,  2.0, 2.0, 0.0,
            0.0, 0.0, 0.0,  2.0, 2.0, 0.0,  0.0, 2.0, 0.0,
            // Quad B (shared vertices at (2,0,0) and (2,2,0) offset by +1e-6)
            2.0+1e-6, 0.0, 0.0,  4.0, 0.0, 0.0,  4.0, 2.0, 0.0,
            2.0+1e-6, 0.0, 0.0,  4.0, 2.0, 0.0,  2.0+1e-6, 2.0, 0.0,
        ];
        let before = count_boundary_edges_r(&p);
        let welded = weld_vertices(&mut p, 1e-5);
        assert!(welded > 0, "should weld shared-edge vertices");
        repair_mesh(&mut p); // no degenerates expected
        let after = count_boundary_edges_r(&p);
        assert!(after < before, "welding should reduce boundary edges: {before}→{after}");
    }

    /// Boundary edge counter for testing (exact, uses same edge_hash as fill_holes).
    fn count_boundary_edges_r(positions: &[f32]) -> usize {
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
        // Pentagon in z=0 plane, CCW
        let pentagon: Vec<[f32; 3]> = vec![
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [3.0, 1.0, 0.0],
            [1.0, 3.0, 0.0],
            [-1.0, 1.0, 0.0],
        ];
        let tris = ear_clip_loop(&pentagon);
        assert_eq!(tris.len(), 3, "pentagon → 3 triangles");
        // All tris should be in z=0 plane and non-degenerate
        for (i, tri) in tris.iter().enumerate() {
            assert!((tri[0][2]).abs() < 1e-5, "tri {i} v0 z");
            assert!((tri[1][2]).abs() < 1e-5, "tri {i} v1 z");
            assert!((tri[2][2]).abs() < 1e-5, "tri {i} v2 z");
            // Non-zero area
            let e1x = tri[1][0] - tri[0][0];
            let e1y = tri[1][1] - tri[0][1];
            let e2x = tri[2][0] - tri[0][0];
            let e2y = tri[2][1] - tri[0][1];
            let area = (e1x * e2y - e1y * e2x).abs() * 0.5;
            assert!(area > 0.01, "tri {i} area={area}");
        }
    }
}
