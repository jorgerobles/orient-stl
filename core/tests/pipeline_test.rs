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

/// Convert stl_io mesh to flat triangle-soup array.
fn stl_to_flat(stl_mesh: &stl_io::IndexedMesh) -> Vec<f32> {
    let mut flat = Vec::with_capacity(stl_mesh.faces.len() * 9);
    for f in &stl_mesh.faces {
        let v0 = stl_mesh.vertices[f.vertices[0]];
        let v1 = stl_mesh.vertices[f.vertices[1]];
        let v2 = stl_mesh.vertices[f.vertices[2]];
        flat.extend_from_slice(&[v0[0], v0[1], v0[2]]);
        flat.extend_from_slice(&[v1[0], v1[1], v1[2]]);
        flat.extend_from_slice(&[v2[0], v2[1], v2[2]]);
    }
    flat
}

/// Compute face normals from vertex positions using cross product (winding-dependent).
fn face_normals_flat(flat: &[f32]) -> Vec<f32> {
    let tris = flat.len() / 9;
    let mut out = Vec::with_capacity(tris * 3);
    for i in 0..tris {
        let b = i * 9;
        let v0 = [flat[b], flat[b + 1], flat[b + 2]];
        let v1 = [flat[b + 3], flat[b + 4], flat[b + 5]];
        let v2 = [flat[b + 6], flat[b + 7], flat[b + 8]];
        let e1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
        let e2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];
        let n = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if len > 1e-12 {
            out.push(n[0] / len);
            out.push(n[1] / len);
            out.push(n[2] / len);
        } else {
            out.push(0.0); out.push(0.0); out.push(1.0);
        }
    }
    out
}

/// Cuenta overhangs (normal pointing down) en el cuartil superior en Z.
fn count_upper_overhangs(flat: &[f32], normals: &[f32]) -> (u32, u32) {
    let tris = flat.len() / 9;
    if tris == 0 { return (0, 0); }

    let mut z_min = f32::MAX;
    let mut z_max = f32::MIN;
    for i in 0..tris {
        let b = i * 9;
        for j in 0..3 {
            let z = flat[b + j * 3 + 2];
            if z < z_min { z_min = z; }
            if z > z_max { z_max = z; }
        }
    }
    let z_cut = z_max - (z_max - z_min) * 0.25;
    let cos_crit = (45.0_f32 * std::f32::consts::PI / 180.0).cos();

    let mut suspicious = 0u32;
    let mut upper = 0u32;
    for i in 0..tris {
        let b = i * 9;
        let cz = (flat[b + 2] + flat[b + 5] + flat[b + 8]) / 3.0;
        if cz < z_cut { continue; }
        upper += 1;
        let bn = i * 3;
        let dot = normals[bn + 2]; // dot with (0,0,1)
        if dot < -cos_crit {
            suspicious += 1;
        }
    }
    (suspicious, upper)
}

/// Test: winding normalization no debe aumentar los overhangs en el cuartil superior.
/// Si lo hiciera, significaría que está volteando triángulos en la dirección incorrecta.
#[test]
fn winding_normalization_does_not_worsen_upper_overhangs() {
    let paths = [
        ("worm", "../resources/Skulled_Wurm_Bird_WOBase.stl"),
        ("broken", "../broken.stl"),
    ];
    for (label, path) in &paths {
        let bytes = std::fs::read(path).unwrap();
        let mut cursor = std::io::Cursor::new(&bytes);
        let stl_mesh = stl_io::read_stl(&mut cursor).unwrap();
        let mut flat = stl_to_flat(&stl_mesh);

        let _removed = orient_core::repair::repair_mesh(&mut flat);

        // Antes de normalize_winding
        let n_before = face_normals_flat(&flat);
        let (sus_before, up_before) = count_upper_overhangs(&flat, &n_before);

        // Después de normalize_winding
        let flipped = orient_core::repair::normalize_winding(&mut flat);
        let n_after = face_normals_flat(&flat);
        let (sus_after, up_after) = count_upper_overhangs(&flat, &n_after);

        println!(
            "[{label}] upper overhangs: {sus_before}/{up_before} → {sus_after}/{up_after} (flipped={flipped})"
        );

        assert!(
            sus_after <= sus_before + 5,
            "[{label}] normalize_winding INCREASED upper overhangs {sus_before}→{sus_after} (+{})",
            sus_after.saturating_sub(sus_before)
        );
    }
}
