fn write_binary_stl(triangles: &[[[f32; 3]; 3]]) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&[0u8; 80]);
    let count = triangles.len() as u32;
    buf.extend_from_slice(&count.to_le_bytes());
    for tri in triangles {
        buf.extend_from_slice(&0f32.to_le_bytes());
        buf.extend_from_slice(&0f32.to_le_bytes());
        buf.extend_from_slice(&0f32.to_le_bytes());
        for v in tri {
            buf.extend_from_slice(&v[0].to_le_bytes());
            buf.extend_from_slice(&v[1].to_le_bytes());
            buf.extend_from_slice(&v[2].to_le_bytes());
        }
        buf.extend_from_slice(&0u16.to_le_bytes());
    }
    buf
}

#[test]
fn profile_real_stl() {
    let bytes = std::fs::read("../resources/Skulled_Wurm_Bird_WOBase.stl").unwrap();
    println!("File: {} bytes ({} MB)", bytes.len(), bytes.len() / 1024 / 1024);

    let t = std::time::Instant::now();
    let mut cursor = std::io::Cursor::new(&bytes);
    let stl_mesh = stl_io::read_stl(&mut cursor).unwrap();
    let read_triangles = stl_mesh.faces.len();
    println!("Parse: {:.2}s — {} faces", t.elapsed().as_secs_f64(), read_triangles);

    let triangles: Vec<[[f32; 3]; 3]> = stl_mesh.faces.iter().map(|f| {
        let v1 = stl_mesh.vertices[f.vertices[0]];
        let v2 = stl_mesh.vertices[f.vertices[1]];
        let v3 = stl_mesh.vertices[f.vertices[2]];
        [[v1[0], v1[1], v1[2]], [v2[0], v2[1], v2[2]], [v3[0], v3[1], v3[2]]]
    }).collect();
    drop(stl_mesh);

    let rewritten = write_binary_stl(&triangles);
    println!("Rewritten: {} bytes", rewritten.len());
    assert_eq!(rewritten.len(), bytes.len());
    println!("Roundtrip OK");
}
