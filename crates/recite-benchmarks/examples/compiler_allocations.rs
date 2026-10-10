//! Full compilation allocation churn; prepared fixtures and result disposal are excluded.
use std::alloc::System;
use std::fs;
use std::hint::black_box;
use std::path::PathBuf;

use recite_benchmarks::BenchmarkFixture;
use recite_benchmarks::compiler::CompilerProject;
use recite_benchmarks::project::BenchmarkProject;
use recite_benchmarks::runtime_allocations::RuntimeAllocationStats;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let capture_dir = std::env::var_os("RECITE_BENCH_CAPTURE_DIR").map(PathBuf::from);
    if let Some(directory) = &capture_dir {
        fs::create_dir_all(directory)?;
    }
    let mut operations = Vec::new();
    for fixture in BenchmarkFixture::selected_from_env()? {
        let project = BenchmarkProject::load_fixture(fixture)?;
        let compiler = CompilerProject::load(&project)?;
        let region = Region::new(GLOBAL);
        let output = compiler.compile_with_schema()?;
        black_box(&output);
        let stats = RuntimeAllocationStats::from(region.change());
        let asset = output.asset();
        if let Some(directory) = &capture_dir {
            let stem = fixture.asset_stem();
            fs::write(
                directory.join(format!("{stem}.recitec")),
                &asset.messagepack,
            )?;
            fs::write(
                directory.join(format!("{stem}.json")),
                asset.inspection_json.as_bytes(),
            )?;
        }
        operations.push(serde_json::json!({
            "fixture": fixture.as_str(),
            "operation": "compile_with_schema",
            "asset_bytes": asset.messagepack.len(),
            "asset_hash": blake3::hash(&asset.messagepack).to_hex().to_string(),
            "inspection_json_bytes": asset.inspection_json.len(),
            "inspection_json_hash": blake3::hash(asset.inspection_json.as_bytes()).to_hex().to_string(),
            "stats": stats,
        }));
    }
    println!("{}", serde_json::to_string_pretty(&operations)?);
    Ok(())
}
