#[path = "../benches/support/sessions.rs"]
mod sessions;

use sessions::{SESSION_HISTORY_SIZES, SESSION_SOURCE_COUNTS, SessionWorkload};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};
use std::alloc::System;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[test]
fn warm_traversal_does_not_clone_accumulated_effect_requests()
-> Result<(), Box<dyn std::error::Error>> {
    let workload = SessionWorkload::new(true)?;
    for history in SESSION_HISTORY_SIZES {
        let checkpoint = workload.adapter_checkpoint(history)?;
        let mut adapter = workload.restore_adapter(&checkpoint)?;
        let checkpoint = workload.preview_checkpoint(history)?;
        let mut preview = workload.restore_preview(&checkpoint)?;
        drop(checkpoint);
        // First growth after restore is allowed to reserve history capacity.
        // Timing/allocation setup includes neither that growth nor old outputs.
        for _ in 0..2 {
            workload.adapter_cycle(&mut adapter)?;
            workload.preview_cycle(&mut preview)?;
        }
        let region = Region::new(GLOBAL);
        let adapter_output = workload.adapter_cycle(&mut adapter)?;
        let adapter_cost = region.change();
        let region = Region::new(GLOBAL);
        let preview_output = workload.preview_cycle(&mut preview)?;
        let preview_cost = region.change();
        eprintln!("history={history} adapter={adapter_cost:?} preview={preview_cost:?}");
        // Measured on this fixed fixture: 13 allocations / 2,944 bytes for
        // adapters; 80 allocations / 20,994 + 24 * history bytes for preview.
        assert!(adapter_cost.allocations <= 16, "{adapter_cost:?}");
        assert!(adapter_cost.bytes_allocated <= 3_500, "{adapter_cost:?}");
        assert!(preview_cost.allocations <= 88, "{preview_cost:?}");
        let projection_bytes = history * std::mem::size_of::<recite_core::ChoiceId>();
        assert!(
            preview_cost.bytes_allocated <= 24_000 + projection_bytes,
            "{preview_cost:?}"
        );
        assert!(!adapter_output.is_empty());
        assert_eq!(
            preview_output.state().selected_choice_history().len(),
            history + 3
        );
        assert_eq!(preview_output.state().deferred_effects().len(), history + 4);
        assert_eq!(
            adapter.snapshot()?,
            recite_runtime::snapshot::encode_session_messagepack(preview.session())?
        );
    }
    for source_count in SESSION_SOURCE_COUNTS {
        let workload = SessionWorkload::with_source_count(false, source_count)?;
        let checkpoint = workload.adapter_checkpoint(0)?;
        let mut adapter = workload.restore_adapter(&checkpoint)?;
        let region = Region::new(GLOBAL);
        let output = workload.adapter_cycle(&mut adapter)?;
        let cost = region.change();
        eprintln!("source_count={source_count} cost={cost:?}");
        assert!(cost.allocations <= 16, "{cost:?}");
        assert!(cost.bytes_allocated <= 3_500, "{cost:?}");
        assert!(!output.is_empty());
    }
    Ok(())
}
