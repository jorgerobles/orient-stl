use std::collections::{HashMap, HashSet, VecDeque};

#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

/// Edge key for adjacency tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Edge {
    a: u32,
    b: u32,
}

impl Edge {
    fn new(a: u32, b: u32) -> Self {
        if a < b { Self { a, b } } else { Self { a: b, b: a } }
    }
}

/// Fix face normals by propagating orientation from a seed triangle.
/// Returns number of flipped triangles.
pub fn fix_normals(_vertices: &mut [f32], indices: &mut [u32]) -> u32 {
    if indices.len() < 3 {
        return 0;
    }
    
    let tri_count = indices.len() / 3;
    let mut adjacency: HashMap<Edge, Vec<u32>> = HashMap::new();
    let mut visited = vec![false; tri_count];
    
    // Build edge adjacency
    for (i, tri) in indices.chunks(3).enumerate() {
        for j in 0..3 {
            let edge = Edge::new(tri[j], tri[(j + 1) % 3]);
            adjacency.entry(edge).or_default().push(i as u32);
        }
    }
    
    let mut flips = 0;
    
    // BFS from each unvisited triangle
    for start in 0..tri_count {
        if visited[start] {
            continue;
        }
        
        let mut queue = VecDeque::new();
        queue.push_back(start);
        visited[start] = true;
        
        while let Some(current) = queue.pop_front() {
            let base = current * 3;
            let tri = [indices[base], indices[base + 1], indices[base + 2]];
            
            // Check each edge for adjacency
            for j in 0..3 {
                let edge = Edge::new(tri[j], tri[(j + 1) % 3]);
                
                if let Some(neighbors) = adjacency.get(&edge) {
                    for &neighbor in neighbors {
                        let ni = neighbor as usize;
                        if visited[ni] {
                            continue;
                        }
                        
                        let nbase = ni * 3;
                        let ntri = [indices[nbase], indices[nbase + 1], indices[nbase + 2]];
                        
                        // Check if neighbor shares edge in opposite direction
                        // If shared vertices are in same order, normals are inconsistent
                        let shared_same_order = (tri[j] == ntri[0] && tri[(j + 1) % 3] == ntri[1]) ||
                            (tri[j] == ntri[1] && tri[(j + 1) % 3] == ntri[2]) ||
                            (tri[j] == ntri[2] && tri[(j + 1) % 3] == ntri[0]);
                        
                        if shared_same_order {
                            // Flip the neighbor triangle
                            indices.swap(nbase + 1, nbase + 2);
                            flips += 1;
                        }
                        
                        visited[ni] = true;
                        queue.push_back(ni);
                    }
                }
            }
        }
    }
    
    flips
}

/// Remove isolated surfaces (connected components) below area threshold.
/// Returns number of removed triangles.
pub fn remove_isolated_surfaces(
    vertices: &mut Vec<f32>,
    indices: &mut Vec<u32>,
    area_threshold: f32,
) -> u32 {
    if indices.is_empty() {
        return 0;
    }
    
    let tri_count = indices.len() / 3;
    let mut adjacency: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut visited = vec![false; tri_count];
    
    // Build vertex adjacency
    for (i, tri) in indices.chunks(3).enumerate() {
        for &v in tri {
            adjacency.entry(v).or_default().push(i as u32);
        }
    }
    
    // Find connected components
    let mut components: Vec<Vec<u32>> = Vec::new();
    
    for start in 0..tri_count {
        if visited[start] {
            continue;
        }
        
        let mut component = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(start);
        visited[start] = true;
        
        while let Some(current) = queue.pop_front() {
            component.push(current as u32);
            let base = current * 3;
            
            for &v in &indices[base..base + 3] {
                if let Some(neighbors) = adjacency.get(&v) {
                    for &neighbor in neighbors {
                        if !visited[neighbor as usize] {
                            visited[neighbor as usize] = true;
                            queue.push_back(neighbor as usize);
                        }
                    }
                }
            }
        }
        
        components.push(component);
    }
    
    // Calculate area for each component in parallel, then remove small ones
    #[cfg(not(target_arch = "wasm32"))]
    let areas: Vec<f32> = components
        .par_iter()
        .map(|component| compute_component_area(vertices, indices, component))
        .collect();

    #[cfg(target_arch = "wasm32")]
    let areas: Vec<f32> = components
        .iter()
        .map(|component| compute_component_area(vertices, indices, component))
        .collect();
    
    let mut removed = 0;
    let mut triangles_to_remove: HashSet<u32> = HashSet::new();
    
    for (component, &area) in components.iter().zip(&areas) {
        if area < area_threshold {
            for &tri_idx in component {
                triangles_to_remove.insert(tri_idx);
            }
            removed += component.len() as u32;
        }
    }
    
    if !triangles_to_remove.is_empty() {
        let new_indices: Vec<u32> = indices
            .chunks(3)
            .enumerate()
            .filter(|(i, _)| !triangles_to_remove.contains(&(*i as u32)))
            .flat_map(|(_, tri)| tri.iter().copied())
            .collect();
        
        *indices = new_indices;
    }
    
    removed
}

fn compute_component_area(vertices: &[f32], indices: &[u32], component: &[u32]) -> f32 {
    component
        .iter()
        .map(|&tri_idx| {
            let base = tri_idx as usize * 3;
            let v0 = [
                vertices[indices[base] as usize * 3],
                vertices[indices[base] as usize * 3 + 1],
                vertices[indices[base] as usize * 3 + 2],
            ];
            let v1 = [
                vertices[indices[base + 1] as usize * 3],
                vertices[indices[base + 1] as usize * 3 + 1],
                vertices[indices[base + 1] as usize * 3 + 2],
            ];
            let v2 = [
                vertices[indices[base + 2] as usize * 3],
                vertices[indices[base + 2] as usize * 3 + 1],
                vertices[indices[base + 2] as usize * 3 + 2],
            ];
            
            let edge1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
            let edge2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];
            let cross = [
                edge1[1] * edge2[2] - edge1[2] * edge2[1],
                edge1[2] * edge2[0] - edge1[0] * edge2[2],
                edge1[0] * edge2[1] - edge1[1] * edge2[0],
            ];
            let len = (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
            len * 0.5
        })
        .sum()
}

/// Remove invisible (degenerate) surfaces.
/// Returns number of removed triangles.
pub fn remove_invisible_surfaces(
    vertices: &mut Vec<f32>,
    indices: &mut Vec<u32>,
) -> u32 {
    // Compute area for each triangle in parallel
    #[cfg(not(target_arch = "wasm32"))]
    let areas: Vec<f32> = indices
        .chunks(3)
        .par_bridge()
        .map(|tri| {
            let v0 = [
                vertices[tri[0] as usize * 3],
                vertices[tri[0] as usize * 3 + 1],
                vertices[tri[0] as usize * 3 + 2],
            ];
            let v1 = [
                vertices[tri[1] as usize * 3],
                vertices[tri[1] as usize * 3 + 1],
                vertices[tri[1] as usize * 3 + 2],
            ];
            let v2 = [
                vertices[tri[2] as usize * 3],
                vertices[tri[2] as usize * 3 + 1],
                vertices[tri[2] as usize * 3 + 2],
            ];
            
            let edge1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
            let edge2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];
            let cross = [
                edge1[1] * edge2[2] - edge1[2] * edge2[1],
                edge1[2] * edge2[0] - edge1[0] * edge2[2],
                edge1[0] * edge2[1] - edge1[1] * edge2[0],
            ];
            (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt() * 0.5
        })
        .collect();

    #[cfg(target_arch = "wasm32")]
    let areas: Vec<f32> = indices
        .chunks(3)
        .map(|tri| {
            let v0 = [
                vertices[tri[0] as usize * 3],
                vertices[tri[0] as usize * 3 + 1],
                vertices[tri[0] as usize * 3 + 2],
            ];
            let v1 = [
                vertices[tri[1] as usize * 3],
                vertices[tri[1] as usize * 3 + 1],
                vertices[tri[1] as usize * 3 + 2],
            ];
            let v2 = [
                vertices[tri[2] as usize * 3],
                vertices[tri[2] as usize * 3 + 1],
                vertices[tri[2] as usize * 3 + 2],
            ];
            
            let edge1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
            let edge2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];
            let cross = [
                edge1[1] * edge2[2] - edge1[2] * edge2[1],
                edge1[2] * edge2[0] - edge1[0] * edge2[2],
                edge1[0] * edge2[1] - edge1[1] * edge2[0],
            ];
            (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt() * 0.5
        })
        .collect();
    
    let mut removed = 0;
    let mut new_indices = Vec::with_capacity(indices.len());
    
    for (tri, &area) in indices.chunks(3).zip(&areas) {
        if area > 1e-10 {
            new_indices.extend_from_slice(tri);
        } else {
            removed += 1;
        }
    }
    
    *indices = new_indices;
    removed
}

/// Fill boundary holes using ear-clipping triangulation.
/// Returns number of holes filled.
pub fn fill_boundary_holes(
    _vertices: &mut Vec<f32>,
    indices: &mut Vec<u32>,
    max_groups: u32,
) -> u32 {
    if indices.len() < 3 {
        return 0;
    }
    
    // Find boundary edges (edges appearing only once)
    let mut edge_count: HashMap<Edge, u32> = HashMap::new();
    
    for tri in indices.chunks(3) {
        for j in 0..3 {
            let edge = Edge::new(tri[j], tri[(j + 1) % 3]);
            *edge_count.entry(edge).or_insert(0) += 1;
        }
    }
    
    let boundary_edges: Vec<Edge> = edge_count
        .iter()
        .filter(|(_, &count)| count == 1)
        .map(|(&edge, _)| edge)
        .collect();
    
    if boundary_edges.is_empty() {
        return 0;
    }
    
    // Build adjacency: for each boundary vertex, which boundary edges touch it
    let mut adj: HashMap<u32, Vec<Edge>> = HashMap::new();
    for &e in &boundary_edges {
        adj.entry(e.a).or_default().push(e);
        adj.entry(e.b).or_default().push(e);
    }
    
    // Group boundary edges into loops
    let mut loops: Vec<Vec<u32>> = Vec::new();
    let mut used_edges: HashSet<Edge> = HashSet::new();
    let mut groups_created = 0;
    
    for &start_edge in &boundary_edges {
        if used_edges.contains(&start_edge) || groups_created >= max_groups {
            continue;
        }
        
        let mut loop_verts = Vec::new();
        let current_edge = start_edge;
        
        // Push first edge's vertices — direction doesn't matter for the first edge
        loop_verts.push(current_edge.a);
        loop_verts.push(current_edge.b);
        used_edges.insert(current_edge);
        
        // Walk the loop from current_edge.b
        let mut current_vertex = current_edge.b;
        
        loop {
            // Find next unused edge sharing current_vertex
            let next_edge = adj.get(&current_vertex)
                .and_then(|edges| {
                    edges.iter().find(|&&e| !used_edges.contains(&e))
                })
                .copied();
            
            match next_edge {
                Some(e) => {
                    used_edges.insert(e);
                    let next_vertex = if e.a == current_vertex { e.b } else { e.a };
                    loop_verts.push(next_vertex);
                    current_vertex = next_vertex;
                    
                    // Check if loop is closed
                    if current_vertex == loop_verts[0] {
                        break;
                    }
                }
                None => break,
            }
        }
        
        if loop_verts.len() >= 3 {
            // loop_verts already has unique vertices in order
            loops.push(loop_verts);
            groups_created += 1;
        }
    }
    
    // Triangulate all loops in parallel
    #[cfg(not(target_arch = "wasm32"))]
    let loop_tris: Vec<Vec<u32>> = loops
        .par_iter()
        .map(|loop_verts| {
            let mut tris = Vec::new();
            if loop_verts.len() >= 3 {
                for i in 1..loop_verts.len() - 1 {
                    tris.push(loop_verts[0]);
                    tris.push(loop_verts[i]);
                    tris.push(loop_verts[i + 1]);
                }
            }
            tris
        })
        .collect();

    #[cfg(target_arch = "wasm32")]
    let loop_tris: Vec<Vec<u32>> = loops
        .iter()
        .map(|loop_verts| {
            let mut tris = Vec::new();
            if loop_verts.len() >= 3 {
                for i in 1..loop_verts.len() - 1 {
                    tris.push(loop_verts[0]);
                    tris.push(loop_verts[i]);
                    tris.push(loop_verts[i + 1]);
                }
            }
            tris
        })
        .collect();
    
    let filled: u32 = loop_tris.iter().map(|t| t.len() as u32 / 3).sum();
    
    let mut new_indices = indices.to_vec();
    for tris in &loop_tris {
        new_indices.extend_from_slice(tris);
    }
    
    *indices = new_indices;
    filled
}
