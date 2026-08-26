use std::io::Read;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).expect("Usage: diag <stl-file>");
    
    let mut f = std::fs::File::open(path).unwrap();
    let mut bytes = Vec::new();
    f.read_to_end(&mut bytes).unwrap();
    
    println!("=== Flat-array repair pipeline diagnostic ===\n");
    
    // Parse
    let mut pos = stl_parse(&bytes);
    println!("After parse:       {} tris, {} boundary edges", pos.len() / 9, count_boundary(&pos));
    
    // Repair (dedup)
    geometry_kernel::flat::repair_mesh(&mut pos);
    println!("After dedup:       {} tris, {} boundary edges", pos.len() / 9, count_boundary(&pos));
    
    // Normalize winding
    let flipped = geometry_kernel::flat::normalize_winding(&mut pos);
    println!("After winding ({flipped} flipped): {} tris, {} boundary edges", pos.len() / 9, count_boundary(&pos));
    
    // Weld
    let welded = geometry_kernel::flat::weld_vertices(&mut pos, 1e-5);
    println!("After weld ({welded} snapped): {} tris, {} boundary edges", pos.len() / 9, count_boundary(&pos));
    
    // Post-weld dedup
    geometry_kernel::flat::repair_mesh(&mut pos);
    println!("After dedup2:      {} tris, {} boundary edges", pos.len() / 9, count_boundary(&pos));
    
    // Fill holes with increasing limits
    for limit in [64, 128, 256, 512, 1024] {
        let mut p2 = pos.clone();
        let added = geometry_kernel::flat::fill_holes(&mut p2, limit);
        println!("fill(limit={limit}): +{added} tris => {} total, {} boundary", p2.len() / 9, count_boundary(&p2));
    }
}

fn count_boundary(positions: &[f32]) -> u32 {
    geometry_kernel::flat::count_boundary_edges(positions)
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
