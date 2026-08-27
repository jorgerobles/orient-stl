use crate::types::{ContactPoint, RaftGeometry};

/// Compute the 2D convex hull of a set of points using Andrew's monotone chain algorithm.
fn convex_hull(points: &[[f32; 2]]) -> Vec<[f32; 2]> {
    if points.len() < 3 {
        return points.to_vec();
    }

    let mut sorted = points.to_vec();
    sorted.sort_by(|a, b| {
        a[0].partial_cmp(&b[0])
            .unwrap()
            .then(a[1].partial_cmp(&b[1]).unwrap())
    });

    let mut lower: Vec<[f32; 2]> = Vec::new();
    for &p in &sorted {
        while lower.len() >= 2 {
            let q: [f32; 2] = lower[lower.len() - 2];
            let r: [f32; 2] = lower[lower.len() - 1];
            let cross = (r[0] - q[0]) * (p[1] - q[1]) - (r[1] - q[1]) * (p[0] - q[0]);
            if cross <= 0.0 {
                lower.pop();
            } else {
                break;
            }
        }
        lower.push(p);
    }

    let mut upper: Vec<[f32; 2]> = Vec::new();
    for &p in sorted.iter().rev() {
        while upper.len() >= 2 {
            let q: [f32; 2] = upper[upper.len() - 2];
            let r: [f32; 2] = upper[upper.len() - 1];
            let cross = (r[0] - q[0]) * (p[1] - q[1]) - (r[1] - q[1]) * (p[0] - q[0]);
            if cross <= 0.0 {
                upper.pop();
            } else {
                break;
            }
        }
        upper.push(p);
    }

    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

/// Bowyer-Watson Delaunay triangulation.
fn delaunay_triangulation(points: &[[f32; 2]]) -> Vec<[u32; 3]> {
    if points.len() < 3 {
        return Vec::new();
    }

    // Create super triangle that encompasses all points
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for &p in points {
        min_x = min_x.min(p[0]);
        max_x = max_x.max(p[0]);
        min_y = min_y.min(p[1]);
        max_y = max_y.max(p[1]);
    }

    let dx = max_x - min_x;
    let dy = max_y - min_y;
    let dmax = dx.max(dy);
    let midx = (min_x + max_x) / 2.0;
    let midy = (min_y + max_y) / 2.0;

    // All vertices: first 3 are super triangle, rest are input points
    let mut all_vertices: Vec<[f32; 2]> = vec![
        [midx - 20.0 * dmax, midy - dmax],
        [midx, midy + 20.0 * dmax],
        [midx + 20.0 * dmax, midy - dmax],
    ];
    for &p in points {
        all_vertices.push(p);
    }

    // Initialize triangles with super triangle (indices 0, 1, 2)
    let mut triangles: Vec<[usize; 3]> = vec![[0, 1, 2]];

    // Add each input point (indices start at 3)
    for point_idx in 3..all_vertices.len() {
        let p = all_vertices[point_idx];
        let mut bad_triangles: Vec<usize> = Vec::new();

        // Find all triangles whose circumcircle contains the point
        for (i, tri) in triangles.iter().enumerate() {
            let a = all_vertices[tri[0]];
            let b = all_vertices[tri[1]];
            let c = all_vertices[tri[2]];

            if point_in_circumcircle(p, a, b, c) {
                bad_triangles.push(i);
            }
        }

        // Find boundary of polygonal hole
        let mut polygon: Vec<[usize; 2]> = Vec::new();

        for &i in &bad_triangles {
            let tri = triangles[i];
            for edge in [[tri[0], tri[1]], [tri[1], tri[2]], [tri[2], tri[0]]] {
                let mut shared = false;
                for &j in &bad_triangles {
                    if i == j {
                        continue;
                    }
                    let other = triangles[j];
                    if (other[0] == edge[0] || other[1] == edge[0] || other[2] == edge[0])
                        && (other[0] == edge[1] || other[1] == edge[1] || other[2] == edge[1])
                    {
                        shared = true;
                        break;
                    }
                }
                if !shared {
                    polygon.push(edge);
                }
            }
        }

        // Remove bad triangles (in reverse order to preserve indices)
        for &i in bad_triangles.iter().rev() {
            triangles.remove(i);
        }

        // Add new triangles connecting polygon edges to the new point
        for edge in &polygon {
            triangles.push([edge[0], edge[1], point_idx]);
        }
    }

    // Remove triangles that share vertices with super triangle (indices 0, 1, 2)
    let super_vert_count = 3usize;
    let mut result = Vec::new();
    for tri in &triangles {
        if tri[0] < super_vert_count || tri[1] < super_vert_count || tri[2] < super_vert_count {
            continue;
        }
        result.push([
            (tri[0] - super_vert_count) as u32,
            (tri[1] - super_vert_count) as u32,
            (tri[2] - super_vert_count) as u32,
        ]);
    }

    result
}

/// Check if a point is inside the circumcircle of a triangle.
fn point_in_circumcircle(p: [f32; 2], a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> bool {
    let ax = a[0] - p[0];
    let ay = a[1] - p[1];
    let bx = b[0] - p[0];
    let by = b[1] - p[1];
    let cx = c[0] - p[0];
    let cy = c[1] - p[1];

    let det = (ax * ax + ay * ay) * (bx * cy - cx * by)
        - (bx * bx + by * by) * (ax * cy - cx * ay)
        + (cx * cx + cy * cy) * (ax * by - bx * ay);

    det > 0.0
}

/// Kruskal's MST with union-find for line connections.
fn minimum_spanning_tree(edges: &[(usize, usize, f32)]) -> Vec<(usize, usize)> {
    if edges.is_empty() {
        return Vec::new();
    }

    let n = edges.iter().map(|&(a, b, _)| a.max(b)).max().unwrap() + 1;
    let mut parent: Vec<usize> = (0..n).collect();
    let mut rank = vec![0; n];

    fn find(parent: &mut [usize], x: usize) -> usize {
        if parent[x] != x {
            parent[x] = find(parent, parent[x]);
        }
        parent[x]
    }

    fn union(parent: &mut [usize], rank: &mut [usize], x: usize, y: usize) -> bool {
        let x_root = find(parent, x);
        let y_root = find(parent, y);

        if x_root == y_root {
            return false;
        }

        if rank[x_root] < rank[y_root] {
            parent[x_root] = y_root;
        } else if rank[x_root] > rank[y_root] {
            parent[y_root] = x_root;
        } else {
            parent[y_root] = x_root;
            rank[x_root] += 1;
        }

        true
    }

    // Sort edges by weight
    let mut sorted_edges = edges.to_vec();
    sorted_edges.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());

    let mut mst = Vec::new();
    for &(a, b, _) in &sorted_edges {
        if union(&mut parent, &mut rank, a, b) {
            mst.push((a, b));
        }
    }

    mst
}

/// Generate a line-connected raft from contact points.
///
/// Algorithm:
/// 1. Compute convex hull of contact base points (2D)
/// 2. Delaunay triangulation → raft mesh triangles
/// 3. MST + extra edges → line connections
/// 4. Output: vertices (flat f32), triangle indices, line indices
pub fn generate_raft(
    contacts: &[ContactPoint],
    _config: &crate::types::SupportConfig,
) -> RaftGeometry {
    if contacts.is_empty() {
        return RaftGeometry {
            vertices: Vec::new(),
            triangles: Vec::new(),
            lines: Vec::new(),
        };
    }

    // Extract 2D base points
    let base_points: Vec<[f32; 2]> = contacts
        .iter()
        .map(|c| [c.base[0], c.base[1]])
        .collect();

    // Compute convex hull
    let hull = convex_hull(&base_points);

    if hull.len() < 3 {
        // Degenerate case: all points collinear or single point
        // Just create a minimal raft connecting the points
        let mut vertices = Vec::new();
        for &p in &base_points {
            vertices.push(p[0]);
            vertices.push(p[1]);
            vertices.push(contacts[0].base[2]); // use z from first contact
        }

        let mut lines = Vec::new();
        for i in 0..base_points.len() {
            for j in (i + 1)..base_points.len() {
                lines.push(i as u32);
                lines.push(j as u32);
            }
        }

        return RaftGeometry {
            vertices,
            triangles: Vec::new(),
            lines,
        };
    }

    // Delaunay triangulation of hull points
    let triangles = delaunay_triangulation(&hull);

    // Create vertices array (flat [x,y,z, ...])
    let mut vertices = Vec::new();
    let z = contacts[0].base[2];
    for &p in &hull {
        vertices.push(p[0]);
        vertices.push(p[1]);
        vertices.push(z);
    }

    // Create line connections using MST
    let mut edges: Vec<(usize, usize, f32)> = Vec::new();
    for i in 0..hull.len() {
        for j in (i + 1)..hull.len() {
            let dx = hull[i][0] - hull[j][0];
            let dy = hull[i][1] - hull[j][1];
            let dist = (dx * dx + dy * dy).sqrt();
            edges.push((i, j, dist));
        }
    }

    let mst = minimum_spanning_tree(&edges);

    // Add some extra edges from Delaunay for better connectivity
    let mut lines: Vec<u32> = Vec::new();
    let mut line_set: std::collections::HashSet<(u32, u32)> = std::collections::HashSet::new();

    // Add MST edges
    for (a, b) in &mst {
        let key = (*a as u32).min(*b as u32);
        let key2 = (*a as u32).max(*b as u32);
        line_set.insert((key, key2));
        lines.push(*a as u32);
        lines.push(*b as u32);
    }

    // Add some Delaunay edges for visual appeal
    for tri in &triangles {
        for edge in &[[tri[0], tri[1]], [tri[1], tri[2]], [tri[2], tri[0]]] {
            let key = edge[0].min(edge[1]);
            let key2 = edge[0].max(edge[1]);
            if !line_set.contains(&(key, key2)) {
                line_set.insert((key, key2));
                lines.push(edge[0]);
                lines.push(edge[1]);
            }
        }
    }

    // Convert triangle indices (Delaunay indices are relative to hull points)
    let triangle_indices: Vec<u32> = triangles.iter().flat_map(|t| t.iter().copied()).collect();

    RaftGeometry {
        vertices,
        triangles: triangle_indices,
        lines,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ContactPoint, SupportType};

    fn test_contacts() -> Vec<ContactPoint> {
        vec![
            ContactPoint {
                position: [0.0, 0.0, 0.0],
                base: [0.0, 0.0, -1.0],
                support_type: SupportType::Light,
                tip_diameter: 0.25,
                penetration: 0.2,
            },
            ContactPoint {
                position: [10.0, 0.0, 0.0],
                base: [10.0, 0.0, -1.0],
                support_type: SupportType::Light,
                tip_diameter: 0.25,
                penetration: 0.2,
            },
            ContactPoint {
                position: [5.0, 10.0, 0.0],
                base: [5.0, 10.0, -1.0],
                support_type: SupportType::Light,
                tip_diameter: 0.25,
                penetration: 0.2,
            },
        ]
    }

    #[test]
    fn generate_raft_produces_geometry() {
        let contacts = test_contacts();
        let config = crate::types::SupportConfig::default();

        let raft = generate_raft(&contacts, &config);

        assert!(!raft.vertices.is_empty(), "raft should have vertices");
        assert!(!raft.lines.is_empty(), "raft should have line connections");
    }

    #[test]
    fn single_contact_produces_minimal_raft() {
        let contacts = vec![ContactPoint {
            position: [5.0, 5.0, 0.0],
            base: [5.0, 5.0, -1.0],
            support_type: SupportType::Light,
            tip_diameter: 0.25,
            penetration: 0.2,
        }];
        let config = crate::types::SupportConfig::default();

        let raft = generate_raft(&contacts, &config);

        // Single point should have one vertex
        assert_eq!(raft.vertices.len(), 3);
    }

    #[test]
    fn two_contacts_produce_connection() {
        let contacts = vec![
            ContactPoint {
                position: [0.0, 0.0, 0.0],
                base: [0.0, 0.0, -1.0],
                support_type: SupportType::Light,
                tip_diameter: 0.25,
                penetration: 0.2,
            },
            ContactPoint {
                position: [10.0, 0.0, 0.0],
                base: [10.0, 0.0, -1.0],
                support_type: SupportType::Light,
                tip_diameter: 0.25,
                penetration: 0.2,
            },
        ];
        let config = crate::types::SupportConfig::default();

        let raft = generate_raft(&contacts, &config);

        // Two points should have a line connection
        assert!(!raft.lines.is_empty());
        assert_eq!(raft.lines.len(), 2); // one line = two indices
    }

    #[test]
    fn empty_contacts_produce_empty_raft() {
        let config = crate::types::SupportConfig::default();

        let raft = generate_raft(&[], &config);

        assert!(raft.vertices.is_empty());
        assert!(raft.triangles.is_empty());
        assert!(raft.lines.is_empty());
    }

    #[test]
    fn convex_hull_basic() {
        let points = vec![
            [0.0, 0.0],
            [10.0, 0.0],
            [10.0, 10.0],
            [0.0, 10.0],
            [5.0, 5.0], // interior point
        ];

        let hull = convex_hull(&points);

        // Hull should have 4 corners
        assert_eq!(hull.len(), 4);

        // Interior point should not be in hull
        assert!(!hull.iter().any(|&p| (p[0] - 5.0).abs() < 0.1 && (p[1] - 5.0).abs() < 0.1));
    }

    #[test]
    fn raft_vertices_are_valid() {
        let contacts = test_contacts();
        let config = crate::types::SupportConfig::default();

        let raft = generate_raft(&contacts, &config);

        // Check all vertices are finite
        for v in raft.vertices.chunks(3) {
            assert!(v[0].is_finite());
            assert!(v[1].is_finite());
            assert!(v[2].is_finite());
        }
    }

    #[test]
    fn mst_connects_all_vertices() {
        let edges = vec![
            (0, 1, 1.0),
            (1, 2, 2.0),
            (0, 2, 3.0),
        ];

        let mst = minimum_spanning_tree(&edges);

        // MST should have n-1 edges for n vertices
        assert_eq!(mst.len(), 2);
    }
}
