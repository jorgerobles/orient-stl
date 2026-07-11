use std::io::Cursor;

pub(crate) fn parse_stl(bytes: &[u8]) -> Result<Vec<[f32; 3]>, String> {
    let mut cursor = Cursor::new(bytes);
    let mesh = stl_io::read_stl(&mut cursor).map_err(|e| format!("STL parse error: {e}"))?;

    let triangle_count = mesh.faces.len();
    if triangle_count > 5_000_000 {
        return Err(format!(
            "STL has {triangle_count} triangles, exceeds maximum of 5,000,000"
        ));
    }

    let mut positions = Vec::with_capacity(triangle_count * 3);
    for face in &mesh.faces {
        let v1 = mesh.vertices[face.vertices[0]];
        let v2 = mesh.vertices[face.vertices[1]];
        let v3 = mesh.vertices[face.vertices[2]];
        positions.push([v1[0], v1[1], v1[2]]);
        positions.push([v2[0], v2[1], v2[2]]);
        positions.push([v3[0], v3[1], v3[2]]);
    }

    Ok(positions)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_binary_stl(triangles: &[[[f32; 3]; 3]]) -> Vec<u8> {
        let mut buf = Vec::new();
        // 80-byte header
        buf.extend_from_slice(&[0u8; 80]);
        // triangle count as u32 LE
        let count = triangles.len() as u32;
        buf.extend_from_slice(&count.to_le_bytes());
        for tri in triangles {
            // normal (unused, set to zero)
            buf.extend_from_slice(&0f32.to_le_bytes());
            buf.extend_from_slice(&0f32.to_le_bytes());
            buf.extend_from_slice(&0f32.to_le_bytes());
            for v in tri {
                buf.extend_from_slice(&v[0].to_le_bytes());
                buf.extend_from_slice(&v[1].to_le_bytes());
                buf.extend_from_slice(&v[2].to_le_bytes());
            }
            // attribute byte count (u16)
            buf.extend_from_slice(&0u16.to_le_bytes());
        }
        buf
    }

    #[test]
    fn parse_two_triangles() {
        let triangles = [
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            [[1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]],
        ];
        let buf = write_binary_stl(&triangles);
        let result = parse_stl(&buf).unwrap();
        assert_eq!(result.len(), 6);
        assert_eq!(result[0], [0.0, 0.0, 0.0]);
        assert_eq!(result[4], [1.0, 1.0, 0.0]);
    }

    #[test]
    fn empty_stl() {
        let buf = write_binary_stl(&[]);
        let result = parse_stl(&buf).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn invalid_stl_returns_error() {
        let buf = vec![0u8; 10];
        assert!(parse_stl(&buf).is_err());
    }
}
