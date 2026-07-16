//! orient-stl native CLI — evaluate and rank STL orientations from the command line.
//!
//! The pipeline mirrors the WASM worker path:
//!   1. Parse binary STL, precompute mesh, compute convex hull
//!   2. Generate candidate build directions
//!   3. Per candidate: score, shadowed-overhang, stability, yaw quaternion
//!   4. Normalize bounds, rank, select diverse subset
//!   5. Output JSON summary
//!
//! Build:  `cargo build --features cli --profile release`
//! Run:    `cargo run --features cli -- --stl ../test-tetrahedron.stl`

use std::path::PathBuf;

use clap::Parser;
use orient_core::decimate::sample_for_hull;
use orient_core::hull;
use orient_core::mesh::MeshData;
use orient_core::ranking::{CandidateMetrics, ScoreWeights, rank_by_consensus, rank_by_topsis, rank_by_weights, to_display_score};
use orient_core::scoring;
use orient_core::selection;
use orient_core::stability;
use orient_core::yaw;
use orient_core::{normalise_dir, prepare_data_native, reconstruct_mesh, repair};

// ---------------------------------------------------------------------------
// CLI arguments
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(name = "orient", version, about = "STL orientation analyser")]
struct Args {
    /// Path to a binary STL file
    #[arg(short, long)]
    stl: PathBuf,

    /// Candidate generation mode: "hull" or "hull_plus_sphere"
    #[arg(long, default_value = "hull")]
    mode: String,

    /// Deduplication angle in degrees
    #[arg(long, default_value_t = 3.0)]
    dedupe_angle: f32,

    /// Critical overhang angle in degrees
    #[arg(long, default_value_t = 30.0)]
    critical_angle: f32,

    /// Hill-climb refinement iterations (0 = skip)
    #[arg(long, default_value_t = 50)]
    refine_iters: u32,

    /// Ranking method: "weights", "consensus", or "topsis"
    #[arg(long, default_value_t = String::from("weights"))]
    method: String,

    /// Six weights: overhang footprint cross surface height shadowed
    #[arg(long, default_value = "1.0,1.0,1.0,1.0,1.0,1.0", value_parser = parse_weights)]
    weights: [f32; 6],

    /// Maximum candidates in the final diverse subset
    #[arg(long, default_value_t = 20)]
    max_candidates: usize,

    /// Minimum angle between selected candidates (degrees)
    #[arg(long, default_value_t = 15.0)]
    min_angle: f32,

    /// Whether to exclude unstable candidates
    #[arg(long, default_value_t = true)]
    exclude_unstable: bool,

    /// Output JSON path (omit for stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Score once, then rank with all 8 profiles × 3 rankers (deterministic)
    #[arg(long)]
    all_rankings: bool,

    /// Prepend identity direction [0,-1,0] (as-loaded orientation) to candidates
    #[arg(long)]
    with_identity: bool,

    /// Axis convention: "y-up" or "z-up" (applies swap to mesh before scoring)
    #[arg(long, default_value_t = String::from("z-up"))]
    convention: String,

    /// Skip mesh repair (weld vertices, cull slivers, remove duplicates)
    #[arg(long)]
    no_repair: bool,
}

fn parse_weights(s: &str) -> Result<[f32; 6], String> {
    let v: Vec<f32> = s.split(',').map(|x| x.trim().parse::<f32>().map_err(|e| format!("{e}"))).collect::<Result<Vec<_>, _>>()?;
    if v.len() != 6 {
        return Err(format!("expected 6 comma-separated weights, got {}", v.len()));
    }
    let mut out = [0.0f32; 6];
    out.copy_from_slice(&v);
    Ok(out)
}

// ---------------------------------------------------------------------------
// Profile presets (mirrors web/src/profiles/*.json)
// ---------------------------------------------------------------------------

const PROFILES: &[(&str, [f32; 6])] = &[
    ("overhang-only",       [1.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
    ("footprint-only",      [0.0, 1.0, 0.0, 0.0, 0.0, 0.0]),
    ("cross-only",          [0.0, 0.0, 1.0, 0.0, 0.0, 0.0]),
    ("surface-only",        [0.0, 0.0, 0.0, 1.0, 0.0, 0.0]),
    ("height-only",         [0.0, 0.0, 0.0, 0.0, 1.0, 0.0]),
    ("overhang-footprint",  [1.0, 1.0, 0.0, 0.0, 0.0, 0.5]),
    ("equal",               [1.0, 1.0, 1.0, 1.0, 1.0, 1.0]),
    ("resin-biased",        [0.5, 1.0, 2.0, 0.5, 0.5, 2.0]),
];

const RANKERS: &[&str] = &["weights", "consensus", "topsis"];

// ---------------------------------------------------------------------------
// Output schema
// ---------------------------------------------------------------------------

#[derive(serde::Serialize)]
struct CliOutput {
    meta: Meta,
    candidates: Vec<CandidateOut>,
    selected: Vec<usize>,
    rankings: Vec<RankingEntry>,
}

#[derive(serde::Serialize)]
struct RankingEntry {
    candidate: usize,
    profile: String,
    ranker: String,
    composite_score: f32,
    rank: usize,
}

#[derive(serde::Serialize)]
struct Meta {
    stl: String,
    triangle_count: usize,
    candidate_count: usize,
    method: String,
    weights: [f32; 6],
    critical_angle_deg: f32,
    refine_iters: u32,
    exclude_unstable: bool,
}

#[derive(serde::Serialize)]
struct CandidateOut {
    index: usize,
    direction: [f32; 3],
    quaternion: [f32; 4],
    overhang: f32,
    footprint: f32,
    max_cross: f32,
    surface: f32,
    height: f32,
    shadowed: f32,
    stable: bool,
    stability_margin: f32,
    contact_area: f32,
    composite_score: f32,
}

// ---------------------------------------------------------------------------
// Pipeline helpers
// ---------------------------------------------------------------------------

/// Compute metrics for one direction. Mirrors the `score_all_directions` loop.
fn score_one(
    dir: &[f32; 3],
    mesh: &MeshData,
    critical_angle_deg: f32,
    refine_iters: u32,
) -> ([f32; 4], [f32; 3], scoring::ScoreComponents, f32, stability::StabilityResult) {
    let (nd, _) = normalise_dir(*dir);

    let final_dir = if refine_iters > 0 {
        let seed = orient_core::rng::seed_from_direction(&nd, 0);
        let rng = orient_core::rng::Rng::new(seed);
        let (best, _) = orient_core::refine_once(&mesh, &nd, critical_angle_deg, refine_iters.min(500), rng);
        best
    } else {
        nd
    };

    let c = scoring::score_components(&final_dir, mesh, critical_angle_deg, 64);
    let shadowed = c.shadowed;
    let stab = stability::check_stability(&final_dir, mesh);
    let q = yaw::full_quaternion(&final_dir, mesh);

    (q, final_dir, c, shadowed, stab)
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() -> Result<(), String> {
    let args = Args::parse();

    // 1. Read STL
    let bytes = std::fs::read(&args.stl).map_err(|e| format!("Cannot read {}: {e}", args.stl.display()))?;

    // 2. Parse, precompute, generate candidates
    let mut od = prepare_data_native(&bytes, &args.mode, args.dedupe_angle)?;

    // Optional mesh repair (on by default)
    if !args.no_repair {
        let old_tris = od.normals.len() / 3;
        repair::repair_mesh(&mut od.positions, &mut od.normals, &mut od.areas);
        let new_tris = od.normals.len() / 3;
        if old_tris > new_tris {
            od.repair_removed = Some((old_tris - new_tris) as u32);
        }
    }

    // Apply axis convention (z-up → y-up swap)
    if args.convention == "z-up" {
        // new_y = old_z, new_z = -old_y for positions and normals
        for i in (0..od.positions.len()).step_by(3) {
            let old_y = od.positions[i + 1];
            od.positions[i + 1] = od.positions[i + 2];
            od.positions[i + 2] = -old_y;
        }
        for i in (0..od.normals.len()).step_by(3) {
            let old_y = od.normals[i + 1];
            od.normals[i + 1] = od.normals[i + 2];
            od.normals[i + 2] = -old_y;
        }
        for i in (0..od.directions.len()).step_by(3) {
            let old_y = od.directions[i + 1];
            od.directions[i + 1] = od.directions[i + 2];
            od.directions[i + 2] = -old_y;
        }
    }

    let mesh = reconstruct_mesh(&od.positions, &od.normals, &od.areas);

    // 3. Hull (needed for stability)
    let hull_verts = sample_for_hull(&mesh.vertices);
    let hull = hull::compute_hull(&hull_verts);

    // 4. Reconstruct direction list from flat array
    let n_dirs = od.directions.len() / 3;
    let mut dirs: Vec<[f32; 3]> = od.directions
        .chunks_exact(3)
        .map(|c| [c[0], c[1], c[2]])
        .collect();

    // Prepend identity direction (as-loaded orientation)
    if args.with_identity {
        dirs.insert(0, [0.0, -1.0, 0.0]);
    }

    // 5. Score every direction
    let crit = args.critical_angle;
    let mut metrics: Vec<CandidateMetrics> = Vec::with_capacity(n_dirs);
    let mut raw_q: Vec<[f32; 4]> = Vec::with_capacity(n_dirs);
    let mut raw_dir: Vec<[f32; 3]> = Vec::with_capacity(n_dirs);
    let mut raw_stable: Vec<bool> = Vec::with_capacity(n_dirs);

    for d in &dirs {
        let (q, fd, c, shadowed, stab) = score_one(d, &mesh, crit, args.refine_iters);
        raw_q.push(q);
        raw_dir.push(fd);
        raw_stable.push(stab.stable);
        metrics.push(CandidateMetrics {
            overhang: c.overhang,
            footprint: c.footprint,
            max_cross: c.max_cross,
            surface: c.surface_quality,
            height: c.height,
            shadowed,
        });
    }

    // Recompute stability for all candidates
    let mut stab_details: Vec<stability::StabilityResult> = Vec::with_capacity(n_dirs);
    for d in &raw_dir {
        let stab = stability::check_stability(d, &mesh);
        stab_details.push(stab);
    }

    // 6. Rank (one method or all combos)
    let (candidates_out, rankings, selected) = if args.all_rankings {
        let mut rankings = Vec::new();

        // Use equal+weights as the primary ranking for diversity selection
        let primary_w = ScoreWeights { w_overhang: 1.0, w_footprint: 1.0, w_cross: 1.0, w_surface: 1.0, w_height: 1.0, w_shadowed: 1.0 };
        let primary_w_sum: f32 = 6.0;
        let primary_ranked = rank_by_weights(&metrics, &primary_w);

        // Build lookup: candidate index → display score from primary ranking
        let mut display_lookup = vec![0.0f32; dirs.len()];
        for &(idx, raw) in &primary_ranked {
            display_lookup[idx] = to_display_score(raw, "weights", primary_w_sum);
        }

        for (profile_name, pw) in PROFILES {
            let w = ScoreWeights {
                w_overhang: pw[0], w_footprint: pw[1], w_cross: pw[2],
                w_surface: pw[3], w_height: pw[4], w_shadowed: pw[5],
            };
            let pw_sum: f32 = pw.iter().sum();
            for ranker_name in RANKERS {
                let ranked = match *ranker_name {
                    "weights" => rank_by_weights(&metrics, &w),
                    "consensus" => rank_by_consensus(&metrics, &w),
                    "topsis" => rank_by_topsis(&metrics, &w),
                    _ => unreachable!(),
                };
                for (pos, &(idx, score)) in ranked.iter().enumerate() {
                    rankings.push(RankingEntry {
                        candidate: idx,
                        profile: profile_name.to_string(),
                        ranker: ranker_name.to_string(),
                        composite_score: to_display_score(score, ranker_name, pw_sum),
                        rank: pos + 1,
                    });
                }
            }
        }

        // Candidates in index order, with primary ranking display score
        let c_out: Vec<CandidateOut> = (0..dirs.len())
            .map(|i| {
                let m = &metrics[i];
                CandidateOut {
                    index: i,
                    direction: raw_dir[i],
                    quaternion: raw_q[i],
                    overhang: m.overhang,
                    footprint: m.footprint,
                    max_cross: m.max_cross,
                    surface: m.surface,
                    height: m.height,
                    shadowed: m.shadowed,
                    stable: stab_details[i].stable,
                    stability_margin: stab_details[i].margin,
                    contact_area: stab_details[i].contact_area,
                    composite_score: display_lookup[i],
                }
            })
            .collect();

        let scored: Vec<(usize, f32)> = primary_ranked.iter().map(|&(i, s)| (i, s)).collect();
        let sel = selection::merge_candidates(
            &scored, &raw_dir, &raw_stable,
            args.exclude_unstable, args.max_candidates, args.min_angle,
        );

        (c_out, rankings, sel)
    } else {
        let w = ScoreWeights {
            w_overhang: args.weights[0],
            w_footprint: args.weights[1],
            w_cross: args.weights[2],
            w_surface: args.weights[3],
            w_height: args.weights[4],
            w_shadowed: args.weights[5],
        };
        let w_sum: f32 = args.weights.iter().sum();
        let ranked = match args.method.as_str() {
            "weights" => rank_by_weights(&metrics, &w),
            "consensus" => rank_by_consensus(&metrics, &w),
            "topsis" => rank_by_topsis(&metrics, &w),
            other => return Err(format!("Unknown method '{other}'; expected weights, consensus, or topsis")),
        };

        // Build full candidate output (ranked order) with display scores
        let c_out: Vec<CandidateOut> = ranked
            .iter()
            .map(|&(idx, score)| {
                CandidateOut {
                    index: idx,
                    direction: raw_dir[idx],
                    quaternion: raw_q[idx],
                    overhang: metrics[idx].overhang,
                    footprint: metrics[idx].footprint,
                    max_cross: metrics[idx].max_cross,
                    surface: metrics[idx].surface,
                    height: metrics[idx].height,
                    shadowed: metrics[idx].shadowed,
                    stable: stab_details[idx].stable,
                    stability_margin: stab_details[idx].margin,
                    contact_area: stab_details[idx].contact_area,
                    composite_score: to_display_score(score, &args.method, w_sum),
                }
            })
            .collect();

        let scored: Vec<(usize, f32)> = ranked.iter().map(|&(i, s)| (i, s)).collect();
        let sel = selection::merge_candidates(
            &scored, &raw_dir, &raw_stable,
            args.exclude_unstable, args.max_candidates, args.min_angle,
        );

        (c_out, Vec::new(), sel)
    };

    // 7. Assemble output
    let out = CliOutput {
        meta: Meta {
            stl: args.stl.to_string_lossy().to_string(),
            triangle_count: mesh.triangle_count,
            candidate_count: dirs.len(),
            method: args.method,
            weights: args.weights,
            critical_angle_deg: crit,
            refine_iters: args.refine_iters,
            exclude_unstable: args.exclude_unstable,
        },
        candidates: candidates_out,
        rankings,
        selected,
    };

    let json = serde_json::to_string_pretty(&out).map_err(|e| format!("Serialize error: {e}"))?;

    match args.output {
        Some(p) => std::fs::write(&p, &json).map_err(|e| format!("Write error: {e}")),
        None => { println!("{json}"); Ok(()) }
    }
}
