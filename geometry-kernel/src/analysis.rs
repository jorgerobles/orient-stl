use std::collections::{HashMap, VecDeque};
use crate::types::MeshAnalysis;

/// Analyze mesh properties
pub fn analyze_mesh(vertices: &[f32], indices: &[u32]) -> MeshAnalysis {
    let vertex_count = vertices.len() / 3;
    let face_count = indices.len() / 3;
    
    // Count boundary edges
    let mut edge_count: HashMap<(u32, u32), u32> = HashMap::new();
    
    for tri in indices.chunks(3) {
        for j in 0..3 {
            let a = tri[j];
            let b = tri[(j + 1) % 3];
            let key = if a < b { (a, b) } else { (b, a) };
            *edge_count.entry(key).or_insert(0) += 1;
        }
    }
    
    let boundary_edges = edge_count.values().filter(|&&count| count == 1).count() as u32;
    
    // Calculate surface area
    let surface_area: f32 = indices
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
            let len = (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
            len * 0.5
        })
        .sum();
    
    // Check if watertight (no boundary edges)
    let is_watertight = boundary_edges == 0;
    
    // Estimate genus using Euler characteristic: V - E + F = 2 - 2g
    let edges = edge_count.len() as i32;
    let euler_char = vertex_count as i32 - edges + face_count as i32;
    let genus = if is_watertight {
        (2 - euler_char) / 2
    } else {
        0
    };
    
    // Calculate volume (signed, assuming closed mesh)
    let volume = if is_watertight {
        calculate_signed_volume(vertices, indices)
    } else {
        0.0
    };
    
    MeshAnalysis {
        vertex_count,
        face_count,
        boundary_edges,
        genus,
        is_watertight,
        surface_area,
        volume,
    }
}

fn calculate_signed_volume(vertices: &[f32], indices: &[u32]) -> f32 {
    let mut volume = 0.0;
    
    for tri in indices.chunks(3) {
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
        
        // Signed volume of tetrahedron formed with origin
        volume += v0[0] * (v1[1] * v2[2] - v1[2] * v2[1])
            - v0[1] * (v1[0] * v2[2] - v1[2] * v2[0])
            + v0[2] * (v1[0] * v2[1] - v1[1] * v2[0]);
    }
    
    volume / 6.0
}

/// Count connected components
pub fn count_connected_components(_vertices: &[f32], indices: &[u32]) -> u32 {
    if indices.is_empty() {
        return 0;
    }
    
    let tri_count = indices.len() / 3;
    let mut adjacency: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut visited = vec![false; tri_count];
    
    for (i, tri) in indices.chunks(3).enumerate() {
        for &v in tri {
            adjacency.entry(v).or_default().push(i as u32);
        }
    }
    
    let mut components = 0;
    
    for start in 0..tri_count {
        if visited[start] {
            continue;
        }
        
        components += 1;
        let mut queue = VecDeque::new();
        queue.push_back(start);
        visited[start] = true;
        
        while let Some(current) = queue.pop_front() {
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
    }
    
    components
}
