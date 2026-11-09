//! Author-facing captions for preview condition inputs and effect requests.
use recite_runtime::preview::PreviewConditionQuery;
use recite_runtime::{DialogueEffectMode, DialogueEffectRequest};
use recite_writer_session::preview::trace;

pub fn condition(query: &PreviewConditionQuery) -> String {
    format!("Condition input · {}", trace::condition(query))
}
pub fn effect(effect: &DialogueEffectRequest) -> String {
    let mode = match effect.mode {
        DialogueEffectMode::Deferred => "Deferred",
        DialogueEffectMode::Immediate => "Immediate",
        DialogueEffectMode::Blocking => "Blocking",
    };
    format!("{mode} effect request · {}", trace::effect(effect))
}
