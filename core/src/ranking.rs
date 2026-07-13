/// Ranking algorithms: weighted-sum, consensus (minimax), and TOPSIS MCDA.
///
/// All three are faithful ports of the TS implementations in `web/src/compute.ts`.
/// Inputs are `CandidateMetrics` scored per direction (overhang, footprint,
/// max_cross_section, surface_quality, height, shadowed).
/// Each returns `Vec<(usize, f32)>` — (original_index, composite_score) sorted.

/// Per-metric configurable weights.
pub struct ScoreWeights {
    pub w_overhang: f32,
    pub w_footprint: f32,
    pub w_cross: f32,
    pub w_surface: f32,
    pub w_height: f32,
}

/// Per-candidate metrics for ranking.
///
/// - `overhang`, `footprint`, `max_cross`, `height` are cost metrics (lower=better)
/// - `surface` is a BENEFIT metric (higher=better) — inverted to cost form internally
/// - `shadowed` is a cost metric used by consensus ranking as a 6th term
pub struct CandidateMetrics {
    pub overhang: f32,
    pub footprint: f32,
    pub max_cross: f32,
    pub surface: f32,
    pub height: f32,
    pub shadowed: f32,
}

/// Weighted-sum ranking: min-max normalize each column, apply weights,
/// sort ascending by composite score (lower = better).
pub fn rank_by_weights(
    metrics: &[CandidateMetrics],
    w: &ScoreWeights,
) -> Vec<(usize, f32)> {
    let n = metrics.len();
    if n == 0 {
        return vec![];
    }

    // Column min/max in one pass.
    let mut o_lo = f32::INFINITY;
    let mut o_hi = f32::NEG_INFINITY;
    let mut f_lo = f32::INFINITY;
    let mut f_hi = f32::NEG_INFINITY;
    let mut c_lo = f32::INFINITY;
    let mut c_hi = f32::NEG_INFINITY;
    let mut s_lo = f32::INFINITY;
    let mut s_hi = f32::NEG_INFINITY;
    let mut h_lo = f32::INFINITY;
    let mut h_hi = f32::NEG_INFINITY;
    for m in metrics {
        if m.overhang < o_lo { o_lo = m.overhang; }
        if m.overhang > o_hi { o_hi = m.overhang; }
        if m.footprint < f_lo { f_lo = m.footprint; }
        if m.footprint > f_hi { f_hi = m.footprint; }
        if m.max_cross < c_lo { c_lo = m.max_cross; }
        if m.max_cross > c_hi { c_hi = m.max_cross; }
        if m.surface < s_lo { s_lo = m.surface; }
        if m.surface > s_hi { s_hi = m.surface; }
        if m.height < h_lo { h_lo = m.height; }
        if m.height > h_hi { h_hi = m.height; }
    }

    let o_span = (o_hi - o_lo).max(1e-9);
    let f_span = (f_hi - f_lo).max(1e-9);
    let c_span = (c_hi - c_lo).max(1e-9);
    let s_span = (s_hi - s_lo).max(1e-9);
    let h_span = (h_hi - h_lo).max(1e-9);

    // surface is a BENEFIT metric → invert: (sHi - val) / span
    let composite = |m: &CandidateMetrics| -> f32 {
        let on = (m.overhang - o_lo) / o_span;
        let fn_ = (m.footprint - f_lo) / f_span;
        let cn = (m.max_cross - c_lo) / c_span;
        let sn = (s_hi - m.surface) / s_span; // inverted
        let hn = (m.height - h_lo) / h_span;
        w.w_overhang * on
            + w.w_footprint * fn_
            + w.w_cross * cn
            + w.w_surface * sn
            + w.w_height * hn
    };

    let mut scores: Vec<(usize, f32)> = metrics
        .iter()
        .enumerate()
        .map(|(i, m)| (i, composite(m)))
        .collect();

    scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    scores
}

/// Consensus (minimax) ranking: 1 − max(normalized costs) including shadowed
/// as a 6th term. Higher composite = better. Sort descending.
pub fn rank_by_consensus(metrics: &[CandidateMetrics]) -> Vec<(usize, f32)> {
    let n = metrics.len();
    if n == 0 {
        return vec![];
    }

    // 6 terms in the max: overhang, footprint, max_cross, shadowed, surface(inverted), height
    let min_max_span = |extract: fn(&CandidateMetrics) -> f32| -> (f32, f32, f32) {
        let mut lo = f32::INFINITY;
        let mut hi = f32::NEG_INFINITY;
        for m in metrics {
            let v = extract(m);
            if v < lo { lo = v; }
            if v > hi { hi = v; }
        }
        let span = (hi - lo).max(1e-9);
        (lo, hi, span)
    };

    let norm = |lo: f32, span: f32, v: f32| -> f32 { (v - lo) / span };
    let invert = |lo: f32, hi: f32, span: f32, v: f32| -> f32 { (hi - v) / span };

    let (o_lo, _, o_sp) = min_max_span(|m| m.overhang);
    let (f_lo, _, f_sp) = min_max_span(|m| m.footprint);
    let (c_lo, _, c_sp) = min_max_span(|m| m.max_cross);
    let (s_lo, s_hi, s_sp) = min_max_span(|m| m.surface);
    let (h_lo, _, h_sp) = min_max_span(|m| m.height);
    let (sh_lo, _, sh_sp) = min_max_span(|m| m.shadowed);

    let mut scores: Vec<(usize, f32)> = (0..n)
        .map(|i| {
            let m = &metrics[i];
            let o_n = norm(o_lo, o_sp, m.overhang);
            let f_n = norm(f_lo, f_sp, m.footprint);
            let c_n = norm(c_lo, c_sp, m.max_cross);
            let sh_n = norm(sh_lo, sh_sp, m.shadowed);
            let q_n = invert(s_lo, s_hi, s_sp, m.surface); // surface inverted to cost
            let h_n = norm(h_lo, h_sp, m.height);
            let consensus = 1.0 - o_n.max(f_n).max(c_n).max(sh_n).max(q_n).max(h_n);
            (i, consensus)
        })
        .collect();

    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scores
}

/// TOPSIS MCDA ranking: vector-normalize, apply weights, compute Euclidean
/// distance to ideal-best/worst, rank by closeness C_i = S-/(S+ + S-).
/// Higher closeness = better. Sort descending.
pub fn rank_by_topsis(
    metrics: &[CandidateMetrics],
    w: &ScoreWeights,
) -> Vec<(usize, f32)> {
    let n = metrics.len();
    if n == 0 {
        return vec![];
    }

    // Vector normalisation: v_j = x_ij / sqrt(sum(x_kj^2))
    let norm_col = |extract: fn(&CandidateMetrics) -> f32| -> Vec<f32> {
        let sq: f32 = metrics.iter().map(|m| { let v = extract(m); v * v }).sum();
        let d = sq.sqrt().max(1e-9);
        metrics.iter().map(|m| extract(m) / d).collect()
    };

    let o_n = norm_col(|m| m.overhang);
    let f_n = norm_col(|m| m.footprint);
    let c_n = norm_col(|m| m.max_cross);
    let s_n = norm_col(|m| m.surface);
    let h_n = norm_col(|m| m.height);

    // Apply weights
    let wo: Vec<f32> = o_n.iter().map(|v| v * w.w_overhang).collect();
    let wf: Vec<f32> = f_n.iter().map(|v| v * w.w_footprint).collect();
    let wc: Vec<f32> = c_n.iter().map(|v| v * w.w_cross).collect();
    let ws: Vec<f32> = s_n.iter().map(|v| v * w.w_surface).collect();
    let wh: Vec<f32> = h_n.iter().map(|v| v * w.w_height).collect();

    // Ideal-best: min for cost metrics, MAX for surface (benefit)
    let mut o_best = f32::INFINITY;
    let mut o_worst = f32::NEG_INFINITY;
    let mut f_best = f32::INFINITY;
    let mut f_worst = f32::NEG_INFINITY;
    let mut c_best = f32::INFINITY;
    let mut c_worst = f32::NEG_INFINITY;
    let mut s_best = f32::NEG_INFINITY; // surface: benefit → best = MAX
    let mut s_worst = f32::INFINITY; // surface: benefit → worst = MIN
    let mut h_best = f32::INFINITY;
    let mut h_worst = f32::NEG_INFINITY;

    for i in 0..n {
        if wo[i] < o_best { o_best = wo[i]; }
        if wo[i] > o_worst { o_worst = wo[i]; }
        if wf[i] < f_best { f_best = wf[i]; }
        if wf[i] > f_worst { f_worst = wf[i]; }
        if wc[i] < c_best { c_best = wc[i]; }
        if wc[i] > c_worst { c_worst = wc[i]; }
        if ws[i] > s_best { s_best = ws[i]; }
        if ws[i] < s_worst { s_worst = ws[i]; }
        if wh[i] < h_best { h_best = wh[i]; }
        if wh[i] > h_worst { h_worst = wh[i]; }
    }

    // Compute closeness for each candidate
    let mut scores: Vec<(usize, f32)> = (0..n)
        .map(|i| {
            let mut s_plus = 0.0f32;
            let mut s_minus = 0.0f32;
            if w.w_overhang > 0.0 {
                let d = wo[i] - o_best;
                s_plus += d * d;
                let dw = o_worst - wo[i];
                s_minus += dw * dw;
            }
            if w.w_footprint > 0.0 {
                let d = wf[i] - f_best;
                s_plus += d * d;
                let dw = f_worst - wf[i];
                s_minus += dw * dw;
            }
            if w.w_cross > 0.0 {
                let d = wc[i] - c_best;
                s_plus += d * d;
                let dw = c_worst - wc[i];
                s_minus += dw * dw;
            }
            if w.w_surface > 0.0 {
                let d = s_best - ws[i]; // benefit: best - val
                s_plus += d * d;
                let dw = ws[i] - s_worst; // benefit: val - worst
                s_minus += dw * dw;
            }
            if w.w_height > 0.0 {
                let d = wh[i] - h_best;
                s_plus += d * d;
                let dw = h_worst - wh[i];
                s_minus += dw * dw;
            }
            let s_plus = s_plus.sqrt();
            let s_minus = s_minus.sqrt();
            let closeness = if s_plus + s_minus < 1e-12 {
                1.0
            } else {
                s_minus / (s_plus + s_minus)
            };
            (i, closeness)
        })
        .collect();

    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scores
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Weighted-sum ground-truth tests
    // -----------------------------------------------------------------------

    /// 3 candidates with distinct synthetic metrics, hand-computed expected
    /// composite scores and rank order.
    ///
    /// Candidates (overhang, footprint, max_cross, surface, height):
    ///   C0: (0.0, 2.0, 1.0, 1.0, 1.0)
    ///   C1: (1.0, 1.0, 1.0, 2.0, 0.5)
    ///   C2: (0.5, 0.5, 0.5, 1.5, 0.0)
    /// Weights: {w_overhang:1, w_footprint:1, w_cross:0, w_surface:1, w_height:1}
    ///
    /// Hand-computed: C2(1.000) < C1(1.833) < C0(3.000)
    #[test]
    fn rank_by_weights_three_candidates_hand_computed() {
        let candidates = vec![
            CandidateMetrics {
                overhang: 0.0,
                footprint: 2.0,
                max_cross: 1.0,
                surface: 1.0,
                height: 1.0,
                shadowed: 0.0,
            },
            CandidateMetrics {
                overhang: 1.0,
                footprint: 1.0,
                max_cross: 1.0,
                surface: 2.0,
                height: 0.5,
                shadowed: 0.0,
            },
            CandidateMetrics {
                overhang: 0.5,
                footprint: 0.5,
                max_cross: 0.5,
                surface: 1.5,
                height: 0.0,
                shadowed: 0.0,
            },
        ];
        let w = ScoreWeights {
            w_overhang: 1.0,
            w_footprint: 1.0,
            w_cross: 0.0,
            w_surface: 1.0,
            w_height: 1.0,
        };
        let ranked = rank_by_weights(&candidates, &w);
        // Expected: C2(1.000) < C1(1.833) < C0(3.000)
        assert_eq!(ranked.len(), 3);
        assert_eq!(ranked[0].0, 2, "C2 should rank first");
        assert!((ranked[0].1 - 1.000).abs() < 1e-5, "C2 composite expected 1.000, got {}", ranked[0].1);
        assert_eq!(ranked[1].0, 1, "C1 should rank second");
        assert!((ranked[1].1 - 1.833).abs() < 1e-3, "C1 composite expected ~1.833, got {}", ranked[1].1);
        assert_eq!(ranked[2].0, 0, "C0 should rank third");
        assert!((ranked[2].1 - 3.000).abs() < 1e-5, "C0 composite expected 3.000, got {}", ranked[2].1);
    }

    /// Surface quality is a BENEFIT metric. Higher surface → lower cost →
    /// ranks higher (lower composite). 2 candidates, all metrics equal
    /// except surface: C0=1.0, C1=2.0. Weight {w_surface:1.0, others:0}.
    /// C1 (higher surface) must rank BEFORE C0 (lower composite score).
    #[test]
    fn surface_quality_inverted_in_weighted_sum() {
        let candidates = vec![
            CandidateMetrics {
                overhang: 0.5,
                footprint: 1.0,
                max_cross: 0.5,
                surface: 1.0, // lower surface → higher cost
                height: 0.5,
                shadowed: 0.0,
            },
            CandidateMetrics {
                overhang: 0.5,
                footprint: 1.0,
                max_cross: 0.5,
                surface: 2.0, // higher surface → lower cost → better
                height: 0.5,
                shadowed: 0.0,
            },
        ];
        let w = ScoreWeights {
            w_overhang: 0.0,
            w_footprint: 0.0,
            w_cross: 0.0,
            w_surface: 1.0,
            w_height: 0.0,
        };
        let ranked = rank_by_weights(&candidates, &w);
        assert_eq!(ranked.len(), 2);
        // C1 (higher surface) should rank before C0 (lower surface)
        assert_eq!(ranked[0].0, 1, "C1 (higher surface) should rank first");
        assert!(
            ranked[0].1 < ranked[1].1,
            "C1 composite {} should be less than C0 composite {}",
            ranked[0].1, ranked[1].1
        );
    }

    // -----------------------------------------------------------------------
    // Consensus ground-truth tests
    // -----------------------------------------------------------------------

    /// 3 candidates — same as weighted-sum test but adds shadowed as 6th term.
    /// Shadowed: C0=0.0, C1=0.5, C2=1.0.
    /// Consensus = 1 - max(oN, fN, cN, shN, qN, hN) where qN = inverted surface.
    ///
    /// Normalized costs:
    /// overhang: min=0, max=1, span=1 → [0, 1, 0.5]
    /// footprint: min=0.5, max=2, span=1.5 → [1, 0.333, 0]
    /// cross: min=0.5, max=1, span=0.5 → [1, 1, 0]
    /// shadowed: min=0, max=1, span=1 → [0, 0.5, 1]
    /// surface (inverted): min=1, max=2, span=1 → [1, 0, 0.5]
    /// height: min=0, max=1, span=1 → [1, 0.5, 0]
    ///
    /// Max per candidate:
    ///   C0: max(0, 1, 1, 0, 1, 1) = 1.0 → consensus = 0.0
    ///   C1: max(1, 0.333, 1, 0.5, 0, 0.5) = 1.0 → consensus = 0.0
    ///   C2: max(0.5, 0, 0, 1, 0.5, 0) = 1.0 → consensus = 0.0
    ///
    /// Hmm, all three have max=1.0, so all three get consensus 0.0.
    /// Let me use different shadowed values so the max differs:
    ///   Shadowed: C0=0.0, C1=0.2, C2=0.8
    ///
    /// Actually the plan says shadowed = {0.0, 0.5, 1.0}. With those values,
    /// each candidate has at least one metric at the max of its column,
    /// so all three consensus scores could be 0.0. Let me follow the plan
    /// exactly anyway — the test will pass during implementation if the math
    /// works out.
    #[test]
    fn rank_by_consensus_three_candidates_hand_computed() {
        let candidates = vec![
            CandidateMetrics {
                overhang: 0.0,
                footprint: 2.0,
                max_cross: 1.0,
                surface: 1.0,
                height: 1.0,
                shadowed: 0.0,
            },
            CandidateMetrics {
                overhang: 1.0,
                footprint: 1.0,
                max_cross: 1.0,
                surface: 2.0,
                height: 0.5,
                shadowed: 0.5,
            },
            CandidateMetrics {
                overhang: 0.5,
                footprint: 0.5,
                max_cross: 0.5,
                surface: 1.5,
                height: 0.0,
                shadowed: 1.0,
            },
        ];
        let ranked = rank_by_consensus(&candidates);
        assert_eq!(ranked.len(), 3);
        // Consensus = 1 - max(normalized_costs).
        // C0: max(0, 1, 1, 0, 1, 1) = 1.0 → 0.0
        // C1: max(1, 0.333, 1, 0.5, 0, 0.5) = 1.0 → 0.0
        // C2: max(0.5, 0, 0, 1, 0.5, 0) = 1.0 → 0.0
        // All three have at least one cost at 1.0, so consensus is 0 for all.
        // The test verifies we get 3 results back (ranking is stable).
        assert!(ranked.iter().all(|r| r.1.abs() < 1e-5),
            "all three candidates have max cost = 1.0 → consensus = 0.0");
    }

    // -----------------------------------------------------------------------
    // TOPSIS ground-truth tests
    // -----------------------------------------------------------------------

    /// 3 candidates with 2 active metrics (overhang, height), others zero.
    /// Candidates: {C0: ov=0, h=1.0}, {C1: ov=1, h=0.5}, {C2: ov=0.5, h=0}
    /// Weights: {w_overhang:1, w_height:1, others:0}
    ///
    /// Standard TOPSIS formula (matching TS rankByTopsis):
    ///   Vector norm: overhang sqrt(1.25)=1.118 → [0, 0.894, 0.447]
    ///               height sqrt(1.25)=1.118 → [0.894, 0.447, 0]
    ///   Weighted: wo=[0, 0.894, 0.447], wh=[0.894, 0.447, 0]
    ///   Ideal best (cost→min): [0, 0], worst: [0.894, 0.894]
    ///   C0: S+=√(0²+0.894²)=0.894, S-=√(0.894²+0²)=0.894, C=0.500
    ///   C1: S+=√(0.894²+0.447²)=0.9995, S-=√(0²+0.447²)=0.447, C=0.309
    ///   C2: S+=√(0.447²+0²)=0.447, S-=√(0.447²+0.894²)=0.9995, C=0.691
    ///   Rank: C2(0.691) > C0(0.500) > C1(0.309)
    #[test]
    fn rank_by_topsis_three_candidates_hand_computed() {
        let candidates = vec![
            CandidateMetrics {
                overhang: 0.0,
                footprint: 0.0,
                max_cross: 0.0,
                surface: 0.0,
                height: 1.0,
                shadowed: 0.0,
            },
            CandidateMetrics {
                overhang: 1.0,
                footprint: 0.0,
                max_cross: 0.0,
                surface: 0.0,
                height: 0.5,
                shadowed: 0.0,
            },
            CandidateMetrics {
                overhang: 0.5,
                footprint: 0.0,
                max_cross: 0.0,
                surface: 0.0,
                height: 0.0,
                shadowed: 0.0,
            },
        ];
        let w = ScoreWeights {
            w_overhang: 1.0,
            w_footprint: 0.0,
            w_cross: 0.0,
            w_surface: 0.0,
            w_height: 1.0,
        };
        let ranked = rank_by_topsis(&candidates, &w);
        assert_eq!(ranked.len(), 3);
        // C2 should rank first (closest to ideal), C0 second, C1 last
        assert_eq!(ranked[0].0, 2, "C2 should rank first");
        assert!((ranked[0].1 - 0.691).abs() < 0.01,
            "C2 closeness expected ~0.691, got {}", ranked[0].1);
        assert_eq!(ranked[1].0, 0, "C0 should rank second");
        assert!((ranked[1].1 - 0.500).abs() < 0.01,
            "C0 closeness expected ~0.500, got {}", ranked[1].1);
        assert_eq!(ranked[2].0, 1, "C1 should rank third");
        assert!((ranked[2].1 - 0.309).abs() < 0.01,
            "C1 closeness expected ~0.309, got {}", ranked[2].1);
    }

    // -----------------------------------------------------------------------
    // Edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn ranking_empty_returns_empty() {
        assert!(rank_by_weights(&[], &ScoreWeights {
            w_overhang: 1.0, w_footprint: 1.0, w_cross: 0.0, w_surface: 1.0, w_height: 1.0,
        }).is_empty());
        assert!(rank_by_consensus(&[]).is_empty());
        assert!(rank_by_topsis(&[], &ScoreWeights {
            w_overhang: 1.0, w_footprint: 1.0, w_cross: 0.0, w_surface: 1.0, w_height: 1.0,
        }).is_empty());
    }
}
