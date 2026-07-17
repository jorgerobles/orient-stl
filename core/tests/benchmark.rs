//! Regression and performance benchmark for the repair pipeline.
//!
//! Run:  cargo test benchmark -- --nocapture
//!
//! Compares candidate count, boundary edges, and latency between:
//!   - FULL pipeline  (repair → normalize → weld → repair → fill → hull → candidates)
//!   - BASELINE       (no repair, just parse → hull → candidates)

use std::time::Instant;
use orient_core::repair;
use orient_core::prepare_data_native;
use orient_core::prepare_data_native_with_repair;

const NULL: &str = "";

fn read_file(path: &str) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|e| panic!("Cannot read {path}: {e}"))
}

#[test]
fn benchmark_pipeline_regression() {
    let fixtures = [
        ("tetrahedron", "../test-tetrahedron.stl"),
        ("worm",        "../resources/Skulled_Wurm_Bird_WOBase.stl"),
        ("broken",      "../broken.stl"),
    ];

    println!("\n══════════════════════════════════════════════════════════════════════");
    println!("  REGRESSION & PERFORMANCE BENCHMARK");
    println!("  max_hole={MAX_HOLE}  weld_eps={WELD_EPS}");
    println!("══════════════════════════════════════════════════════════════════════");

    for (label, path) in &fixtures {
        let bytes = read_file(path);

        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  {label}  ({path})");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        // ── BASELINE: no repair ──
        println!("\n  ── BASELINE (no repair) ──");
        let t0 = Instant::now();
        let base = prepare_data_native(&bytes, "hull", 3.0)
            .expect("baseline prepare failed");
        let base_time = t0.elapsed();
        println!("  candidates:  {}", base.directions.len() / 3);
        println!("  positions:   {} tris", base.positions.len() / 9);
        println!("  boundary:    {}", repair::count_boundary_edges(&base.positions));
        println!("  total time:  {:.3}s", base_time.as_secs_f64());

        // ── FULL pipeline ──
        println!("\n  ── FULL PIPELINE (repair + normalize + weld + fill) ──");
        let t0 = Instant::now();
        let full = prepare_data_native_with_repair(&bytes, "hull", 3.0, MAX_HOLE, WELD_EPS)
            .expect("full pipeline failed");
        let full_time = t0.elapsed();
        println!("  candidates:  {}", full.directions.len() / 3);
        println!("  positions:   {} tris", full.positions.len() / 9);
        println!("  boundary:    {}", repair::count_boundary_edges(&full.positions));
        println!("  total time:  {:.3}s", full_time.as_secs_f64());

        // ── Comparison ──
        let base_candidates = base.directions.len() / 3;
        let full_candidates = full.directions.len() / 3;
        let base_tris = base.positions.len() / 9;
        let full_tris = full.positions.len() / 9;

        println!("\n  ── REGRESSION CHECK ──");
        println!("  {:<24} {:>10} {:>10} {:>8}", NULL, "baseline", "full", "Δ%");
        println!("  {:<24} {:>10} {:>10} {:>8}", "  ────────────────────────", "────────", "────", "───");
        println!("  {:<24} {:>10} {:>10} {:>7.1}%", "  candidates",
            base_candidates, full_candidates,
            pct_change(base_candidates, full_candidates));
        println!("  {:<24} {:>10} {:>10} {:>7.1}%", "  triangles",
            base_tris, full_tris,
            pct_change(base_tris, full_tris));
        println!("  {:<24} {:>10.3}s {:>10.3}s {:>7.1}%", "  total time",
            base_time.as_secs_f64(), full_time.as_secs_f64(),
            pct_change_f64(base_time.as_secs_f64(), full_time.as_secs_f64()));

        // PASS/FAIL evaluation
        let cand_ok = full_candidates as f64 >= base_candidates as f64 * 0.9;
        let time_ok = full_time.as_secs_f64() <= base_time.as_secs_f64() * 1.20;
        println!("\n  ── VERDICT ──");
        println!("  candidates regressed (>10%): {}", if cand_ok { "✅ PASS" } else { "❌ FAIL" });
        println!("  time increased (>20%):       {}", if time_ok { "✅ PASS" } else { "❌ FAIL" });
    }
}

fn pct_change(a: usize, b: usize) -> f64 {
    if a == 0 { return 0.0; }
    (b as f64 / a as f64 - 1.0) * 100.0
}

fn pct_change_f64(a: f64, b: f64) -> f64 {
    if a == 0.0 { return 0.0; }
    (b / a - 1.0) * 100.0
}

const MAX_HOLE: u32 = 64;
const WELD_EPS: f32 = 1e-5;
