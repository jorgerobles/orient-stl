use std::time::Instant;

fn generate_icosphere(subdivisions: u32) -> (Vec<f32>, Vec<u32>) {
    let phi = (1.0 + 5.0_f32.sqrt()) / 2.0;
    
    let mut vertices: Vec<[f32; 3]> = vec![
        [-1.0,  phi, 0.0],
        [ 1.0,  phi, 0.0],
        [-1.0, -phi, 0.0],
        [ 1.0, -phi, 0.0],
        [0.0, -1.0,  phi],
        [0.0,  1.0,  phi],
        [0.0, -1.0, -phi],
        [0.0,  1.0, -phi],
        [ phi, 0.0, -1.0],
        [ phi, 0.0,  1.0],
        [-phi, 0.0, -1.0],
        [-phi, 0.0,  1.0],
    ];
    
    // Normalize
    for v in &mut vertices {
        let len = (v[0]*v[0] + v[1]*v[1] + v[2]*v[2]).sqrt();
        v[0] /= len; v[1] /= len; v[2] /= len;
    }
    
    let mut faces: Vec<[u32; 3]> = vec![
        [0,11,5], [0,5,1], [0,1,7], [0,7,10], [0,10,11],
        [1,5,9], [5,11,4], [11,10,2], [10,7,6], [7,1,8],
        [3,9,4], [3,4,2], [3,2,6], [3,6,8], [3,8,9],
        [4,9,5], [2,4,11], [6,2,10], [8,6,7], [9,8,1],
    ];
    
    // Subdivide
    let mut midpoint_cache = std::collections::HashMap::<(u32,u32), u32>::new();
    
    let get_midpoint = |a: u32, b: u32, vertices: &mut Vec<[f32;3]>, cache: &mut std::collections::HashMap<(u32,u32), u32>| -> u32 {
        let key = if a < b { (a, b) } else { (b, a) };
        if let Some(&idx) = cache.get(&key) {
            return idx;
        }
        let va = vertices[a as usize];
        let vb = vertices[b as usize];
        let mut mid = [(va[0]+vb[0])/2.0, (va[1]+vb[1])/2.0, (va[2]+vb[2])/2.0];
        let len = (mid[0]*mid[0] + mid[1]*mid[1] + mid[2]*mid[2]).sqrt();
        mid[0] /= len; mid[1] /= len; mid[2] /= len;
        let idx = vertices.len() as u32;
        vertices.push(mid);
        cache.insert(key, idx);
        idx
    };
    
    for _ in 0..subdivisions {
        let mut new_faces = Vec::new();
        midpoint_cache.clear();
        for face in &faces {
            let a = face[0]; let b = face[1]; let c = face[2];
            let ab = get_midpoint(a, b, &mut vertices, &mut midpoint_cache);
            let bc = get_midpoint(b, c, &mut vertices, &mut midpoint_cache);
            let ca = get_midpoint(c, a, &mut vertices, &mut midpoint_cache);
            new_faces.push([a, ab, ca]);
            new_faces.push([b, bc, ab]);
            new_faces.push([c, ca, bc]);
            new_faces.push([ab, bc, ca]);
        }
        faces = new_faces;
    }
    
    let positions: Vec<f32> = vertices.iter().flat_map(|v| v.iter().copied()).collect();
    let indices: Vec<u32> = faces.iter().flat_map(|f| f.iter().copied()).collect();
    
    (positions, indices)
}

fn main() {
    println!("Geometry Kernel Benchmark — Rayon vs Sequential\n");
    
    for subdivisions in [2, 3, 4, 5, 6, 7] {
        let (positions, indices) = generate_icosphere(subdivisions);
        let tri_count = indices.len() / 3;
        let vert_count = positions.len() / 3;
        println!("--- {} subdivisions: {} vertices, {} triangles ---", subdivisions, vert_count, tri_count);
        
        // Benchmark fix_normals
        {
            let mut v1 = positions.clone();
            let mut i1 = indices.clone();
            let start = Instant::now();
            let flips = geometry_kernel::fix_normals(&mut v1, &mut i1);
            let elapsed = start.elapsed();
            println!("  fix_normals:          {:?}  (flips: {})", elapsed, flips);
        }
        
        // Benchmark remove_invisible
        {
            let mut v1 = positions.clone();
            let mut i1 = indices.clone();
            let start = Instant::now();
            let removed = geometry_kernel::remove_invisible_surfaces(&mut v1, &mut i1);
            let elapsed = start.elapsed();
            println!("  remove_invisible:     {:?}  (removed: {})", elapsed, removed);
        }
        
        // Benchmark remove_isolated (with a threshold that won't remove anything)
        {
            let mut v1 = positions.clone();
            let mut i1 = indices.clone();
            let start = Instant::now();
            let removed = geometry_kernel::remove_isolated_surfaces(&mut v1, &mut i1, 0.001);
            let elapsed = start.elapsed();
            println!("  remove_isolated:      {:?}  (removed: {})", elapsed, removed);
        }
        
        // Benchmark fill_holes (mesh is watertight, so 0 holes — tests the boundary scan overhead)
        {
            let mut v1 = positions.clone();
            let mut i1 = indices.clone();
            let start = Instant::now();
            let filled = geometry_kernel::fill_boundary_holes(&mut v1, &mut i1, 512);
            let elapsed = start.elapsed();
            println!("  fill_boundary_holes:  {:?}  (filled: {})", elapsed, filled);
        }
        
        // Benchmark analyze
        {
            let start = Instant::now();
            let analysis = geometry_kernel::analyze_mesh(&positions, &indices);
            let elapsed = start.elapsed();
            println!("  analyze_mesh:         {:?}  (watertight: {}, genus: {})", elapsed, analysis.is_watertight, analysis.genus);
        }
        
        println!();
    }
}
