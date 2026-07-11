const HULL_SAMPLE_TARGET: usize = 8000;

pub(crate) fn sample_for_hull(vertices: &[[f32; 3]]) -> Vec<[f32; 3]> {
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
