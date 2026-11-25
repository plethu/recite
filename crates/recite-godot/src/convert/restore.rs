//! Godot dictionaries for restore outcomes.
//!
//! Keys and names follow `recite_adapter::resume_names`, so GDScript sees the
//! same outcome vocabulary as the C ABI report and the conformance matrix.

use godot::builtin::{PackedStringArray, VarDictionary, Variant};
use godot::prelude::ToGodot;

use recite_adapter::resume_names::{
    fallback_reason_landmark, fallback_reason_name, landmark_choices, landmark_kind, landmark_line,
    prompt_resume_name, resume_level_name,
};
use recite_runtime::resume::{ResumeFallback, ResumeFallbackReason, ResumeLandmark, ResumeReport};

use super::core::set_variant;
use super::effects::{effect_dictionary, effects_array};

pub(crate) fn resumed_dictionary(report: &ResumeReport) -> VarDictionary {
    let mut dictionary = VarDictionary::new();
    dictionary.set("outcome", "resumed");
    dictionary.set("level", resume_level_name(report.level));
    dictionary.set("block_id", report.block_id.as_str());
    dictionary.set("prompt", prompt_resume_name(report.prompt));
    dictionary.set("schema_changed", report.schema_changed);
    dictionary.set("compiler_changed", report.compiler_changed);
    dictionary
}

pub(crate) fn fallback_dictionary(fallback: &ResumeFallback) -> VarDictionary {
    let mut dictionary = VarDictionary::new();
    dictionary.set("outcome", "fallback");
    dictionary.set("reason", fallback_reason_name(&fallback.reason));
    // Every key is present, nil when it does not apply, as in the C ABI report.
    let (required, allowed) = match &fallback.reason {
        ResumeFallbackReason::LevelRefused { required, allowed } => (
            resume_level_name(*required).to_variant(),
            resume_level_name(*allowed).to_variant(),
        ),
        _ => (Variant::nil(), Variant::nil()),
    };
    set_variant(&mut dictionary, "required_level", required);
    set_variant(&mut dictionary, "allowed_level", allowed);
    set_variant(
        &mut dictionary,
        "reason_landmark",
        fallback_reason_landmark(&fallback.reason).map_or_else(Variant::nil, |landmark| {
            landmark_dictionary(landmark).to_variant()
        }),
    );
    dictionary.set("block_id", fallback.block_id.as_str());
    dictionary.set("block_available", fallback.block_available);
    set_variant(
        &mut dictionary,
        "landmark",
        fallback
            .landmark
            .as_ref()
            .map_or_else(Variant::nil, |landmark| {
                landmark_dictionary(landmark).to_variant()
            }),
    );
    set_variant(
        &mut dictionary,
        "deferred_effects",
        effects_array(&fallback.deferred_effects).to_variant(),
    );
    set_variant(
        &mut dictionary,
        "pending_effect",
        fallback
            .pending_effect
            .as_ref()
            .map_or_else(Variant::nil, |effect| {
                effect_dictionary(effect).to_variant()
            }),
    );
    let mut history = PackedStringArray::new();
    for choice in &fallback.selected_choice_history {
        history.push(choice.as_str());
    }
    set_variant(
        &mut dictionary,
        "selected_choice_history",
        history.to_variant(),
    );
    set_variant(
        &mut dictionary,
        "locale",
        fallback
            .locale
            .as_ref()
            .map_or_else(Variant::nil, |locale| locale.as_str().to_variant()),
    );
    dictionary
}

fn landmark_dictionary(landmark: &ResumeLandmark) -> VarDictionary {
    let mut dictionary = VarDictionary::new();
    dictionary.set("kind", landmark_kind(landmark));
    set_variant(
        &mut dictionary,
        "line",
        landmark_line(landmark).map_or_else(Variant::nil, |line| line.to_variant()),
    );
    let mut choices = PackedStringArray::new();
    for choice in landmark_choices(landmark) {
        choices.push(choice);
    }
    set_variant(&mut dictionary, "choices", choices.to_variant());
    dictionary
}
