#[path = "../benches/support/sessions.rs"]
mod sessions;

use sessions::{SESSION_HISTORY_SIZES, SessionWorkload};

#[test]
fn session_workloads_preserve_growing_histories_and_deferred_queues_after_restore()
-> Result<(), Box<dyn std::error::Error>> {
    for deferred in [false, true] {
        let workload = SessionWorkload::new(deferred)?;
        for history in SESSION_HISTORY_SIZES {
            let checkpoint = workload.adapter_checkpoint(history)?;
            let mut adapter = workload.restore_adapter(&checkpoint)?;
            workload.adapter_cycle(&mut adapter)?;

            let checkpoint = workload.preview_checkpoint(history)?;
            let mut preview = workload.restore_preview(&checkpoint)?;
            assert_eq!(preview.session().selected_choice_history().len(), history);
            assert_eq!(
                preview.session().deferred_effects().len(),
                if deferred { history + 1 } else { 0 }
            );
            workload.preview_cycle(&mut preview)?;
            assert_eq!(
                adapter.snapshot()?,
                recite_runtime::snapshot::encode_session_messagepack(preview.session())?,
                "adapter drain and preview replay must reach the same checkpoint"
            );
            assert_eq!(
                preview.session().selected_choice_history().len(),
                history + 1
            );
            assert_eq!(
                preview.session().deferred_effects().len(),
                if deferred { history + 2 } else { 0 }
            );
        }
    }
    Ok(())
}
