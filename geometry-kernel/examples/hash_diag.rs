use std::collections::HashMap;
use std::io::Read;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).expect("Usage: hash-diag <stl-file>");
    
    let mut f = std::fs::File::open(path).unwrap();
    let mut bytes = Vec::new();
    f.read_to_end(&mut bytes).unwrap();
    
    let mut pos = stl_parse(&bytes);
    let n0 = pos.len() / 9;
    println!("Loaded {n0} triangles");
    
    let b0 = geometry_kernel::flat::count_boundary_edges(&pos);
    println!("Raw boundary edges: {b0} / {}", n0 * 3);
    
    let w = geometry_kernel::flat::weld_vertices(&mut pos, geometry_kernel::flat::DEFAULT_WELD_EPSILON);
    println!("Welded {w} vertex positions");
    
    let d = geometry_kernel::flat::repair_mesh(&mut pos);
    println!("Removed {d} duplicate triangles");
    let n1 = pos.len() / 9;
    println!("Triangles after dedup: {n1}");
    
    let f = geometry_kernel::flat::normalize_winding(&mut pos);
    println!("Flipped {f} triangles for consistent winding");
    
    let b1 = geometry_kernel::flat::count_boundary_edges(&pos);
    println!("Boundary edges after full repair: {b1} / {}", n1 * 3);
    
    let added = geometry_kernel::flat::fill_holes(&mut pos, geometry_kernel::flat::DEFAULT_MAX_HOLE_EDGES);
    println!("Added {added} fill triangles");
    let n2 = pos.len() / 9;
    
    let b2 = geometry_kernel::flat::count_boundary_edges(&pos);
    println!("Final boundary edges: {b2} / {}", n2 * 3);
    println!("Final triangle count: {n2}");
}

fn stl_parse(bytes: &[u8]) -> Vec<f32> {
    if bytes.len() < 84 { return Vec::new(); }
    let num_triangles = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]) as usize;
    let expected = 84 + num_triangles * 50;
    if bytes.len() < expected { return Vec::new(); }
    let mut positions = Vec::with_capacity(num_triangles * 9);
    for i in 0..num_triangles {
        let base = 84 + i * 50;
        for v in 0..3 {
            let voff = base + 12 + v * 12;
            positions.push(f32::from_le_bytes([bytes[voff], bytes[voff+1], bytes[voff+2], bytes[voff+3]]));
            positions.push(f32::from_le_bytes([bytes[voff+4], bytes[voff+5], bytes[voff+6], bytes[voff+7]]));
            positions.push(f32::from_le_bytes([bytes[voff+8], bytes[voff+9], bytes[voff+10], bytes[voff+11]]));
        }
    }
    positions
}
