//! Encoding may allocate its result and bounded prompt state, not copies of history or sources.
use std::alloc::System;

#[path = "../benches/support/session_fixture.rs"]
mod session_fixture;

use recite_adapter::{SessionDriver, StartRequest};
use recite_runtime::{
    DialogueSessionOptions, LocaleResolution,
    snapshot::{decode_session_messagepack, snapshot_session},
};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[test]
fn encoding_allocates_only_the_output_buffer_and_bounded_prompt_state()
-> Result<(), Box<dyn std::error::Error>> {
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
                let region = Region::new(GLOBAL);
                let encoded = driver.snapshot()?;
                let cost = region.change();
                // Measured fixed cases: five allocations and 378 bytes beyond
                // the result capacity, independent of source/history counts.
                assert!(cost.allocations <= 8, "{cost:?}");
                assert!(cost.bytes_allocated <= encoded.capacity() + 512, "{cost:?}");
                let restored = decode_session_messagepack(asset.dialogue(), &encoded)?;
                let owned = snapshot_session(&restored);
                assert_eq!(owned.sources.len(), sources);
                assert_eq!(owned.selected_choice_history.len(), history);
                assert_eq!(
                    owned.deferred_effects.len(),
                    if deferred { history + 1 } else { 0 }
                );
                assert_eq!(encoded, rmp_serde::to_vec(&owned)?);
            }
        }
    }
    Ok(())
}
