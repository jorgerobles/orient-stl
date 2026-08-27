const HULL_SAMPLE_TARGET: usize = 8000;

pub fn sample_for_hull(vertices: &[[f32; 3]]) -> Vec<[f32; 3]> {
    let n = vertices.len();
    if n <= HULL_SAMPLE_TARGET {
        return vertices.to_vec();
    }

    let mut seen = vec![false; n];

    let extremes = find_extremes(vertices);
    let mut picked: Vec<[f32; 3]> = Vec::with_capacity(HULL_SAMPLE_TARGET);
    for &i in &extremes {
        if !seen[i] {
            seen[i] = true;
            picked.push(vertices[i]);
        }
    }

    let remaining = HULL_SAMPLE_TARGET - picked.len();
    if remaining == 0 { return picked; }

    let step = (n as f64) / (remaining as f64);
    let mut pos = step / 2.0;
    while picked.len() < HULL_SAMPLE_TARGET {
        let idx = pos as usize;
        if idx >= n { break; }
        if !seen[idx] {
            seen[idx] = true;
            picked.push(vertices[idx]);
        }
        pos += step;
    }

    if picked.len() < HULL_SAMPLE_TARGET {
        for i in 0..n {
            if picked.len() >= HULL_SAMPLE_TARGET { break; }
            if !seen[i] {
                seen[i] = true;
                picked.push(vertices[i]);
            }
        }
    }

    picked
}

/// Stride-based triangle decimation for scoring (mirrors web `decimateForScore`).
/// If triangle count ≤ target, returns originals unchanged.
pub fn decimate_for_score(
    positions: &[f32],
    normals: &[f32],
    areas: &[f32],
    target: usize,
) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    let tri_count = normals.len() / 3;
    if tri_count <= target {
        return (positions.to_vec(), normals.to_vec(), areas.to_vec());
    }
    let step = tri_count / target;
    let new_count = (tri_count + step - 1) / step;
    let mut new_pos = Vec::with_capacity(new_count * 9);
    let mut new_norm = Vec::with_capacity(new_count * 3);
    let mut new_area = Vec::with_capacity(new_count);
    for i in 0..new_count {
        let src = i * step;
        let src_n = src * 3;
        let src_p = src * 9;
        new_norm.extend_from_slice(&normals[src_n..src_n + 3]);
        new_area.push(areas[src]);
        new_pos.extend_from_slice(&positions[src_p..src_p + 9]);
    }
    (new_pos, new_norm, new_area)
}

fn find_extremes(vertices: &[[f32; 3]]) -> [usize; 6] {
    let n = vertices.len();
    let mut min_x = 0; let mut max_x = 0;
    let mut min_y = 0; let mut max_y = 0;
    let mut min_z = 0; let mut max_z = 0;
    for i in 1..n {
        if vertices[i][0] < vertices[min_x][0] { min_x = i; }
        if vertices[i][0] > vertices[max_x][0] { max_x = i; }
        if vertices[i][1] < vertices[min_y][1] { min_y = i; }
        if vertices[i][1] > vertices[max_y][1] { max_y = i; }
        if vertices[i][2] < vertices[min_z][2] { min_z = i; }
        if vertices[i][2] > vertices[max_z][2] { max_z = i; }
    }
    [min_x, max_x, min_y, max_y, min_z, max_z]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_mesh_passes_through() {
        let v = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let sampled = sample_for_hull(&v);
        assert_eq!(sampled.len(), 4);
    }

    #[test]
    fn large_mesh_reduces() {
        let mut v = Vec::new();
        for i in 0..20000 {
            v.push([i as f32, i as f32, i as f32]);
        }
        let sampled = sample_for_hull(&v);
        assert!(sampled.len() <= HULL_SAMPLE_TARGET);
    }
}
