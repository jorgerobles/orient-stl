use std::collections::HashSet;

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
}
