use crate::hull::ConvexHull;

pub(crate) fn generate_candidates(hull: &ConvexHull) -> Vec<[f32; 3]> {
    hull.face_normals.clone()
}

pub(crate) fn deduplicate_directions(directions: &[[f32; 3]], angle_deg: f32) -> Vec<[f32; 3]> {
    if directions.is_empty() || angle_deg <= 0.0 {
        return directions.to_vec();
    }
    let cos_threshold = (angle_deg as f64 * std::f64::consts::PI / 180.0).cos() as f32;
    let mut result: Vec<[f32; 3]> = Vec::new();
    'outer: for &dir in directions {
        for &keep in &result {
            let dot = dir[0] * keep[0] + dir[1] * keep[1] + dir[2] * keep[2];
            if dot >= cos_threshold {
                continue 'outer;
            }
        }
        result.push(dir);
    }
    result
}

pub(crate) fn generate_fibonacci_sphere(n: usize) -> Vec<[f32; 3]> {
    let golden = (1.0 + 5.0_f32.sqrt()) * 0.5;
    let mut dirs = Vec::with_capacity(n);
    for i in 0..n {
        let theta = (1.0 - 2.0 * (i as f32 + 0.5) / n as f32).acos();
        let phi = 2.0 * std::f32::consts::PI * i as f32 / golden;
        dirs.push([
            theta.sin() * phi.cos(),
            theta.sin() * phi.sin(),
            theta.cos(),
        ]);
    }
    dirs
}

pub(crate) fn generate_hull_plus_sphere(
    hull: &ConvexHull,
    n: usize,
    dedupe_angle_deg: f32,
) -> Vec<[f32; 3]> {
    let hull_normals = hull.face_normals.clone();
    let deduped_hull = deduplicate_directions(&hull_normals, dedupe_angle_deg);
    let sphere = generate_fibonacci_sphere(n);
    let mut combined = deduped_hull.clone();
    let cos_threshold = (dedupe_angle_deg as f64 * std::f64::consts::PI / 180.0).cos() as f32;
    for &dir in &sphere {
        let mut keep = true;
        for &existing in &deduped_hull {
            let dot = dir[0] * existing[0] + dir[1] * existing[1] + dir[2] * existing[2];
            if dot >= cos_threshold {
                keep = false;
                break;
            }
        }
        if keep {
            combined.push(dir);
        }
    }
    combined
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_hull(normals: Vec<[f32; 3]>) -> ConvexHull {
        ConvexHull {
            face_normals: normals,
        }
    }

    #[test]
    fn generate_from_hull_normals() {
        let hull = make_hull(vec![[0.0, 0.0, 1.0], [0.0, 0.0, -1.0]]);
        let dirs = generate_candidates(&hull);
        assert_eq!(dirs.len(), 2);
    }

    #[test]
    fn dedupe_merges_close_directions() {
        let dirs = vec![[0.0, 0.0, 1.0], [0.0, 0.0349, 0.9994]];
        let deduped = deduplicate_directions(&dirs, 3.0);
        assert_eq!(deduped.len(), 1, "2° separation should merge at 3° threshold");
    }

    #[test]
    fn dedupe_keeps_different_directions() {
        let dirs = vec![[0.0, 0.0, 1.0], [0.0, 0.0698, 0.9976]];
        let deduped = deduplicate_directions(&dirs, 3.0);
        assert_eq!(deduped.len(), 2, "4° separation should NOT merge at 3° threshold");
    }
}
