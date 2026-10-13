use std::alloc::System;
use std::hint::black_box;

use recite_benchmarks::runtime_allocations::RuntimeAllocationStats;
#[path = "../benches/support/sessions.rs"]
mod sessions;

use serde::Serialize;
use sessions::{SESSION_HISTORY_SIZES, SESSION_SOURCE_COUNTS, SessionWorkload};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[derive(Serialize)]
struct Operation {
    operation: &'static str,
    history: usize,
    source_files: usize,
    deferred_effects: bool,
    stats: RuntimeAllocationStats,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut operations = Vec::new();
    for deferred_effects in [false, true] {
        let workload = SessionWorkload::new(deferred_effects)?;
        for history in SESSION_HISTORY_SIZES {
            let checkpoint = workload.adapter_checkpoint(history)?;
            let mut adapter = workload.restore_adapter(&checkpoint)?;
            let region = Region::new(GLOBAL);
            let output = workload.adapter_cycle(&mut adapter)?;
            black_box(&output);
            operations.push(Operation {
                operation: "adapter_choice",
                history,
                source_files: 1,
                deferred_effects,
                stats: region.change().into(),
            });

            let checkpoint = workload.preview_checkpoint(history)?;
            let mut preview = workload.restore_preview(&checkpoint)?;
            let region = Region::new(GLOBAL);
            let output = workload.preview_cycle(&mut preview)?;
            black_box(&output);
            operations.push(Operation {
                operation: "preview_condition_replay",
                history,
                source_files: 1,
                deferred_effects,
                stats: region.change().into(),
            });
        }
    }
    for source_files in SESSION_SOURCE_COUNTS {
        let workload = SessionWorkload::with_source_count(false, source_files)?;
        let checkpoint = workload.adapter_checkpoint(0)?;
        let mut adapter = workload.restore_adapter(&checkpoint)?;
        let region = Region::new(GLOBAL);
        let output = workload.adapter_cycle(&mut adapter)?;
        black_box(&output);
        operations.push(Operation {
            operation: "adapter_source_files",
            history: 0,
            source_files,
            deferred_effects: false,
            stats: region.change().into(),
        });
    }
    println!("{}", serde_json::to_string_pretty(&operations)?);
    Ok(())
}
