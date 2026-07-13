/// Angle-diversity candidate selection.
///
/// `merge_candidates` iterates a pre-sorted list of scored candidates and
/// picks a diverse subset where every retained direction is at least
/// `min_angle_deg` away from all others.
///
/// Faithful port of `web/src/compute.ts:mergeCandidates` (lines 657-759).

/// Select a diverse subset of candidates by angle-diversity filtering.
///
/// `scored` is pre-sorted: weighted-sum ascending by composite, consensus/topsis
/// descending. Iterates in that order, keeps candidates that are at least
/// `min_angle_deg` away from all previously kept directions.
///
/// Returns indices into the original direction/metric arrays that pass the
/// diversity filter, capped at `max_candidates`.
pub(crate) fn merge_candidates(
    _scored: &[(usize, f32)],
    _directions: &[[f32; 3]],
    _stable_flags: &[bool],
    _exclude_unstable: bool,
    _max_candidates: usize,
    _min_angle_deg: f32,
) -> Vec<usize> {
    unimplemented!()
}

/// Angle (in degrees) between two 3D vectors.
pub(crate) fn angle_between(_a: &[f32; 3], _b: &[f32; 3]) -> f32 {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 5 directions at 0°, 10°, 20°, 30°, 40° from [0,0,1] in the XZ plane.
    /// Scoring ascending by index. max_candidates=10, min_angle_deg=15, no
    /// unstable exclusion. Expected: keep 0°, 20°, 40° (each >15° from prior).
    #[test]
    fn merge_candidates_diversity_filter() {
        let scored: Vec<(usize, f32)> = vec![
            (0, 0.1), (1, 0.2), (2, 0.3), (3, 0.4), (4, 0.5),
        ];
        // Directions at 0°, 10°, 20°, 30°, 40° from [0,0,1] in XZ plane.
        let directions: Vec<[f32; 3]> = vec![
            [0.0, 0.0, 1.0],           // 0°
            [0.173648, 0.0, 0.984807], // 10°
            [0.342020, 0.0, 0.939692], // 20°
            [0.5, 0.0, 0.866025],      // 30°
            [0.642787, 0.0, 0.766044], // 40°
        ];
        let stable_flags = vec![true; 5];
        let result = merge_candidates(&scored, &directions, &stable_flags, false, 10, 15.0);
        // Expected: indices [0, 2, 4] — each >15° from the prior kept direction
        assert_eq!(result, vec![0, 2, 4],
            "Diversity filter should keep 0°, 20°, 40° (indices 0, 2, 4)");
    }

    /// Same 5 directions, mark idx 2 as unstable, exclude_unstable=true.
    /// Expected result excludes idx 2, keeping [0, 3, ...].
    #[test]
    fn merge_candidates_excludes_unstable_when_flagged() {
        let scored: Vec<(usize, f32)> = vec![
            (0, 0.1), (1, 0.2), (2, 0.3), (3, 0.4), (4, 0.5),
        ];
        let directions: Vec<[f32; 3]> = vec![
            [0.0, 0.0, 1.0],
            [0.173648, 0.0, 0.984807],
            [0.342020, 0.0, 0.939692],
            [0.5, 0.0, 0.866025],
            [0.642787, 0.0, 0.766044],
        ];
        let stable_flags = vec![true, true, false, true, true];
        let result = merge_candidates(&scored, &directions, &stable_flags, true, 10, 15.0);
        // Expect: 0 kept, 1 too close to 0 (10° < 15°), 2 excluded (unstable),
        // 3 too close to 0 (30° > 15° — wait, 30° > 15° so it IS kept!)
        // Let's trace: 0 kept, 1 -> 10° from 0 = skip, 2 excluded, 3 -> 30° from 0 = keep,
        // 4 -> 40° from 0 AND 20° from 3? 40-30=10 < 15 so skip.
        // Expected: [0, 3]
        assert_eq!(result, vec![0, 3],
            "Unstable filter should exclude idx 2, resulting in [0, 3]");
    }

    /// 10 directions well-separated, max_candidates=3.
    /// Expected result.len() == 3.
    #[test]
    fn merge_candidates_caps_at_max() {
        let scored: Vec<(usize, f32)> = (0..10).map(|i| (i, i as f32 * 0.1)).collect();
        // Directions well-separated on XZ circle (36° apart).
        let directions: Vec<[f32; 3]> = (0..10)
            .map(|i| {
                let angle = (i as f32 * 36.0).to_radians();
                [angle.sin(), 0.0, angle.cos()]
            })
            .collect();
        let stable_flags = vec![true; 10];
        let result = merge_candidates(&scored, &directions, &stable_flags, false, 3, 15.0);
        assert_eq!(result.len(), 3, "Should cap at max_candidates=3");
    }

    /// angle_between orthogonal vectors is exactly 90°.
    #[test]
    fn angle_between_orthogonal_is_90() {
        let a = [1.0, 0.0, 0.0];
        let b = [0.0, 1.0, 0.0];
        let angle = angle_between(&a, &b);
        assert!((angle - 90.0).abs() < 1e-4,
            "Orthogonal vectors should give 90°, got {}", angle);
    }

    /// angle_between parallel vectors is exactly 0°.
    #[test]
    fn angle_between_parallel_is_0() {
        let a = [1.0, 0.0, 0.0];
        let b = [1.0, 0.0, 0.0];
        let angle = angle_between(&a, &b);
        assert!(angle.abs() < 1e-4,
            "Parallel vectors should give 0°, got {}", angle);
    }
}
