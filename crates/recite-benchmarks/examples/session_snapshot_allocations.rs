//! Snapshot source-table and history pressure, separate from wall-clock timing.
use std::alloc::System;

#[path = "../benches/support/session_fixture.rs"]
mod session_fixture;

use recite_adapter::{SessionDriver, StartRequest};
use recite_benchmarks::runtime_allocations::RuntimeAllocationStats;
use recite_runtime::{DialogueSessionOptions, LocaleResolution};
use serde::Serialize;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[derive(Serialize)]
struct SnapshotCost {
    sources: usize,
    history: usize,
    deferred: bool,
    output_bytes: usize,
    output_hash: String,
    stats: RuntimeAllocationStats,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut costs = Vec::new();
    for sources in [1, 4_000] {
        for deferred in [false, true] {
            let (asset, choice) = session_fixture::load(deferred, sources)?;
            for history in [0, 2_048] {
                let mut driver = SessionDriver::new();
                driver.start(
                    StartRequest {
                        asset: &asset,
                        block_id: None,
                        options: DialogueSessionOptions::new(),
                    },
                    &session_fixture::ready,
                    LocaleResolution::new(),
                )?;
                for _ in 0..history {
                    driver.select_choice(
                        choice.clone(),
                        &session_fixture::ready,
                        LocaleResolution::new(),
                    )?;
                }
                let expected = driver.snapshot()?;
                let region = Region::new(GLOBAL);
                let encoded = driver.snapshot()?;
                let stats = region.change().into();
                assert_eq!(encoded, expected);
                costs.push(SnapshotCost {
                    sources,
                    history,
                    deferred,
                    output_bytes: encoded.len(),
                    output_hash: blake3::hash(&encoded).to_hex().to_string(),
                    stats,
                });
            }
        }
    }
    println!("{}", serde_json::to_string_pretty(&costs)?);
    Ok(())
}
