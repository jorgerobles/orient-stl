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
use orient_core::hull::{self, ConvexHull};
use orient_core::mesh::MeshData;
use orient_core::ranking::{CandidateMetrics, ScoreWeights, rank_by_consensus, rank_by_topsis, rank_by_weights};
use orient_core::scoring;
use orient_core::selection;
use orient_core::stability;
use orient_core::yaw;
use orient_core::{normalise_dir, prepare_data_native, reconstruct_mesh};

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
    #[arg(long, default_value_t = 0)]
    refine_iters: u32,

    /// Ranking method: "weights", "consensus", or "topsis"
    #[arg(long, default_value_t = String::from("weights"))]
    method: String,

    /// Five weights: overhang footprint cross surface height
    #[arg(long, default_value = "1.0,1.0,1.0,1.0,1.0", value_parser = parse_weights)]
    weights: [f32; 5],

    /// Maximum candidates in the final diverse subset
    #[arg(long, default_value_t = 10)]
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
}

fn parse_weights(s: &str) -> Result<[f32; 5], String> {
    let v: Vec<f32> = s.split(',').map(|x| x.trim().parse::<f32>().map_err(|e| format!("{e}"))).collect::<Result<Vec<_>, _>>()?;
    if v.len() != 5 {
        return Err(format!("expected 5 comma-separated weights, got {}", v.len()));
    }
    let mut out = [0.0f32; 5];
    out.copy_from_slice(&v);
    Ok(out)
}

// ---------------------------------------------------------------------------
// Output schema
// ---------------------------------------------------------------------------

#[derive(serde::Serialize)]
struct CliOutput {
    meta: Meta,
    candidates: Vec<CandidateOut>,
    selected: Vec<usize>,
}

#[derive(serde::Serialize)]
struct Meta {
    stl: String,
    triangle_count: usize,
    candidate_count: usize,
    method: String,
    weights: [f32; 5],
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
    hull: &ConvexHull,
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
    let shadowed = scoring::shadowed_overhang_fraction(&final_dir, mesh, critical_angle_deg, 32, 0.02);
    let stab = stability::check_stability(&final_dir, mesh, hull);
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
    let od = prepare_data_native(&bytes, &args.mode, args.dedupe_angle)?;
    let mesh = reconstruct_mesh(&od.positions, &od.normals, &od.areas);

    // 3. Hull (needed for stability)
    let hull_verts = sample_for_hull(&mesh.vertices);
    let hull = hull::compute_hull(&hull_verts);

    // 4. Reconstruct direction list from flat array
    let n_dirs = od.directions.len() / 3;
    let dirs: Vec<[f32; 3]> = od.directions
        .chunks_exact(3)
        .map(|c| [c[0], c[1], c[2]])
        .collect();

    // 5. Score every direction
    let crit = args.critical_angle;
    let mut metrics: Vec<CandidateMetrics> = Vec::with_capacity(n_dirs);
    let mut raw_q: Vec<[f32; 4]> = Vec::with_capacity(n_dirs);
    let mut raw_dir: Vec<[f32; 3]> = Vec::with_capacity(n_dirs);
    let mut raw_stable: Vec<bool> = Vec::with_capacity(n_dirs);

    for d in &dirs {
        let (q, fd, c, shadowed, stab) = score_one(d, &mesh, &hull, crit, args.refine_iters);
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

    // 6. Rank
    let w = ScoreWeights {
        w_overhang: args.weights[0],
        w_footprint: args.weights[1],
        w_cross: args.weights[2],
        w_surface: args.weights[3],
        w_height: args.weights[4],
    };
    let ranked = match args.method.as_str() {
        "weights" => rank_by_weights(&metrics, &w),
        "consensus" => rank_by_consensus(&metrics),
        "topsis" => rank_by_topsis(&metrics, &w),
        other => return Err(format!("Unknown method '{other}'; expected weights, consensus, or topsis")),
    };

    // Build full candidate output (ranked order)
    let mut candidates_out: Vec<CandidateOut> = ranked
        .iter()
        .map(|&(idx, score)| {
            let d = raw_dir[idx];
            let q = raw_q[idx];
            let m = &metrics[idx];
            let stab = raw_stable[idx];
            CandidateOut {
                index: idx,
                direction: d,
                quaternion: q,
                overhang: m.overhang,
                footprint: m.footprint,
                max_cross: m.max_cross,
                surface: m.surface,
                height: m.height,
                shadowed: m.shadowed,
                stable: stab,
                stability_margin: 0.0,    // recomputed below if needed
                contact_area: 0.0,
                composite_score: score,
            }
        })
        .collect();

    // Recompute stability details for the top candidates
    // (we stored only the bool above; re-run to get margin/contact for ranked)
    for co in candidates_out.iter_mut() {
        let d = &raw_dir[co.index];
        let stab = stability::check_stability(d, &mesh, &hull);
        co.stable = stab.stable;
        co.stability_margin = stab.margin;
        co.contact_area = stab.contact_area;
    }

    // 7. Select diverse subset
    let scored: Vec<(usize, f32)> = ranked.iter().map(|&(i, s)| (i, s)).collect();
    let selected = selection::merge_candidates(
        &scored, &raw_dir, &raw_stable,
        args.exclude_unstable, args.max_candidates, args.min_angle,
    );

    // 8. Assemble output
    let out = CliOutput {
        meta: Meta {
            stl: args.stl.to_string_lossy().to_string(),
            triangle_count: mesh.triangle_count,
            candidate_count: n_dirs,
            method: args.method,
            weights: args.weights,
            critical_angle_deg: crit,
            refine_iters: args.refine_iters,
            exclude_unstable: args.exclude_unstable,
        },
        candidates: candidates_out,
        selected,
    };

    let json = serde_json::to_string_pretty(&out).map_err(|e| format!("Serialize error: {e}"))?;

    match args.output {
        Some(p) => std::fs::write(&p, &json).map_err(|e| format!("Write error: {e}")),
        None => { println!("{json}"); Ok(()) }
    }
}
