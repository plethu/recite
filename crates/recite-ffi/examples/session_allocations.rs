#[path = "../benches/support/mod.rs"]
mod support;

use std::alloc::System;
use std::hint::black_box;

use serde::Serialize;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, Stats, StatsAlloc};
use support::{HISTORY_SIZES, HostFixture};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[derive(Serialize)]
struct Operation {
    history: usize,
    deferred_effects: bool,
    allocations: usize,
    bytes_allocated: usize,
    reallocations: usize,
    bytes_reallocated: isize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut operations = Vec::new();
    for deferred_effects in [false, true] {
        for history in HISTORY_SIZES {
            let fixture = HostFixture::new(history, deferred_effects);
            let mut session = fixture.prepare();
            let region = Region::new(GLOBAL);
            let batch = session.choose();
            black_box(&batch);
            let Stats {
                allocations,
                bytes_allocated,
                reallocations,
                bytes_reallocated,
                ..
            } = region.change();
            operations.push(Operation {
                history,
                deferred_effects,
                allocations,
                bytes_allocated,
                reallocations,
                bytes_reallocated,
            });
        }
    }
    println!("{}", serde_json::to_string_pretty(&operations)?);
    Ok(())
}
