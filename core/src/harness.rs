//! Spike 002 harness — explore scoring composites for resin orientation.
//! Run: `cargo test harness_run -- --ignored --nocapture`
#![cfg(test)]

use crate::candidates::{deduplicate_directions, generate_candidates};
use crate::decimate::sample_for_hull;
use crate::hull::compute_hull;
use crate::mesh::precompute_mesh;
use crate::ranking::{CandidateMetrics, ScoreWeights, rank_by_weights};
use crate::scoring::{footprint_area, max_cross_section, misalignment_score, min_z_height, score_candidate, shadowed_overhang_fraction};
use crate::stl::parse_stl;

struct WeightCfg {
    name: &'static str,
    w_overhang: f32,
    w_footprint: f32,
    w_cross: f32,
    w_surface: f32,
    w_height: f32,
    w_shadowed: f32,
}

#[ignore]
#[test]
fn harness_run() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let fixtures = [
        (root.join("test-tetrahedron.stl"), "tetrahedron"),
        (root.join("resources/Skulled_Wurm_Bird_WOBase.stl"), "bird"),
    ];
    let configs = [
        WeightCfg { name: "overhang-only (current v1)", w_overhang: 1.0, w_footprint: 0.0, w_cross: 0.0, w_surface: 0.0, w_height: 0.0, w_shadowed: 0.0 },
        WeightCfg { name: "footprint-only (naive)", w_overhang: 0.0, w_footprint: 1.0, w_cross: 0.0, w_surface: 0.0, w_height: 0.0, w_shadowed: 0.0 },
        WeightCfg { name: "cross-only (peel-force)", w_overhang: 0.0, w_footprint: 0.0, w_cross: 1.0, w_surface: 0.0, w_height: 0.0, w_shadowed: 0.0 },
        WeightCfg { name: "surface-only (finish)", w_overhang: 0.0, w_footprint: 0.0, w_cross: 0.0, w_surface: 1.0, w_height: 0.0, w_shadowed: 0.0 },
        WeightCfg { name: "height-only (fast print)", w_overhang: 0.0, w_footprint: 0.0, w_cross: 0.0, w_surface: 0.0, w_height: 1.0, w_shadowed: 0.0 },
        WeightCfg { name: "equal-weights", w_overhang: 1.0, w_footprint: 1.0, w_cross: 1.0, w_surface: 1.0, w_height: 1.0, w_shadowed: 1.0 },
        WeightCfg { name: "resin-biased (cross-heavy)", w_overhang: 0.5, w_footprint: 1.0, w_cross: 2.0, w_surface: 0.5, w_height: 0.5, w_shadowed: 2.0 },
        WeightCfg { name: "overhang+footprint (no cross)", w_overhang: 1.0, w_footprint: 1.0, w_cross: 0.0, w_surface: 0.0, w_height: 0.0, w_shadowed: 0.5 },
    ];
    let bins = 64usize;
    let crit = 30.0f32;

    for (path, label) in fixtures {
        println!("\n================================================================");
        println!("  FIXTURE: {label}");
        println!("================================================================");
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                println!("  (skip: {e})");
                continue;
            }
        };
        println!("  path: {}", path.to_string_lossy());
        let tris = match parse_stl(&bytes) {
            Ok(t) => t,
            Err(e) => { println!("  parse error: {e}"); continue; }
        };
        let flat: Vec<f32> = tris.iter().flat_map(|v| v.iter()).copied().collect();
        let mesh = precompute_mesh(&flat);
        let hull_verts = sample_for_hull(&mesh.vertices);
        let hull = compute_hull(&hull_verts);
        let dirs = deduplicate_directions(&generate_candidates(&hull), 3.0);
        println!("  triangles={}  candidates={}", mesh.triangle_count, dirs.len());

        // Compute raw components per candidate.
        let mut overs = Vec::with_capacity(dirs.len());
        let mut fps = Vec::with_capacity(dirs.len());
        let mut crs = Vec::with_capacity(dirs.len());
        let mut sfs = Vec::with_capacity(dirs.len());
        let mut hts = Vec::with_capacity(dirs.len());
        let mut shs = Vec::with_capacity(dirs.len());
        for d in &dirs {
            overs.push(score_candidate(d, &mesh, crit));
            fps.push(footprint_area(d, &mesh));
            crs.push(max_cross_section(d, &mesh, bins));
            sfs.push(misalignment_score(d, &mesh));
            hts.push(min_z_height(d, &mesh));
            shs.push(shadowed_overhang_fraction(d, &mesh, crit, 32, 0.02));
        }

        // Raw component ranges (for reference).
        println!("\n  raw component ranges (min..max across candidates):");
        println!("    overhang:  {:.3} .. {:.3}", min_of(&overs), max_of(&overs));
        println!("    footprint: {:.3} .. {:.3}", min_of(&fps), max_of(&fps));
        println!("    maxcross:  {:.3} .. {:.3}", min_of(&crs), max_of(&crs));
        println!("    surface:   {:.3} .. {:.3}", min_of(&sfs), max_of(&sfs));
        println!("    height:    {:.3} .. {:.3}", min_of(&hts), max_of(&hts));
        println!("    shadowed:  {:.3} .. {:.3}", min_of(&shs), max_of(&shs));

        // Build CandidateMetrics vector with real shadowed values.
        let metrics: Vec<CandidateMetrics> = (0..dirs.len())
            .map(|i| CandidateMetrics {
                overhang: overs[i],
                footprint: fps[i],
                max_cross: crs[i],
                surface: sfs[i],
                height: hts[i],
                shadowed: shs[i],
            })
            .collect();

        for cfg in &configs {
            let w = ScoreWeights {
                w_overhang: cfg.w_overhang,
                w_footprint: cfg.w_footprint,
                w_cross: cfg.w_cross,
                w_surface: cfg.w_surface,
                w_height: cfg.w_height,
                w_shadowed: cfg.w_shadowed,
            };
            let ranked = rank_by_weights(&metrics, &w);
            if ranked.is_empty() {
                println!("\n  [{}] (no candidates)", cfg.name);
                continue;
            }
            let top = &ranked[0];
            let d = &dirs[top.0];
            println!(
                "\n  [{}] best candidate #{}  score={:.4}",
                cfg.name, top.0, top.1
            );
            println!(
                "    dir=({:+.3},{:+.3},{:+.3})  over={:.3} foot={:.3} cross={:.3} surf={:.3} h={:.3}",
                d[0], d[1], d[2], overs[top.0], fps[top.0], crs[top.0], sfs[top.0], hts[top.0]
            );
            // Top-3 agreement check vs overhang-only baseline.
            let top3: Vec<usize> = ranked.iter().take(3).map(|(i, _)| *i).collect();
            print!("    top-3 candidate ids: {:?}", top3);
            if cfg.name.starts_with("overhang") {
                println!("   (baseline)");
            } else {
                println!();
            }
        }
    }
}

fn min_of(v: &[f32]) -> f32 { v.iter().cloned().fold(f32::INFINITY, f32::min) }
fn max_of(v: &[f32]) -> f32 { v.iter().cloned().fold(f32::NEG_INFINITY, f32::max) }
