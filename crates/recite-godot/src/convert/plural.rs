use godot::builtin::{VarArray, VarDictionary, Variant};
use godot::prelude::ToGodot;

use recite_runtime::{
    DialoguePlural, DialoguePluralResolutionOutcome, PluralResolutionAttempt,
    PluralResolutionOutcome,
};

use super::core::set_variant;

pub(super) fn plural_dictionary(plural: &DialoguePlural) -> VarDictionary {
    let mut dictionary = VarDictionary::new();
    dictionary.set("singular_source_text", plural.singular_source_text.as_str());
    dictionary.set("plural_source_text", plural.plural_source_text.as_str());
    dictionary.set("count", plural.count);
    dictionary.set("selected_arm", plural.selected_arm as i64);
    let mut resolution = VarDictionary::new();
    set_variant(
        &mut resolution,
        "attempts",
        plural
            .resolution
            .attempts
            .iter()
            .map(plural_attempt_dictionary)
            .collect::<VarArray>()
            .to_variant(),
    );
    set_variant(
        &mut resolution,
        "matched_locale",
        plural
            .resolution
            .matched_locale
            .as_deref()
            .map_or_else(Variant::nil, |value| value.to_variant()),
    );
    set_variant(
        &mut resolution,
        "matched_context",
        plural
            .resolution
            .matched_context
            .as_deref()
            .map_or_else(Variant::nil, |value| value.to_variant()),
    );
    set_variant(
        &mut resolution,
        "matched_key",
        plural
            .resolution
            .matched_key
            .as_deref()
            .map_or_else(Variant::nil, |value| value.to_variant()),
    );
    set_variant(
        &mut resolution,
        "matched_arm",
        plural
            .resolution
            .matched_arm
            .map(|arm| (arm as i64).to_variant())
            .unwrap_or_else(Variant::nil),
    );
    set_variant(
        &mut resolution,
        "source_fallback_arm",
        plural
            .resolution
            .source_fallback_arm
            .map(|arm| (arm as i64).to_variant())
            .unwrap_or_else(Variant::nil),
    );
    resolution.set(
        "outcome",
        match plural.resolution.outcome {
            DialoguePluralResolutionOutcome::Translated => "translated",
            DialoguePluralResolutionOutcome::EnglishSourceFallback => "english_source_fallback",
        },
    );
    set_variant(&mut dictionary, "resolution", resolution.to_variant());
    dictionary
}

fn plural_attempt_dictionary(attempt: &PluralResolutionAttempt) -> Variant {
    let mut dictionary = VarDictionary::new();
    dictionary.set("locale", attempt.locale.as_str());
    dictionary.set("context", attempt.context.as_str());
    dictionary.set("key", attempt.key.as_str());
    set_variant(
        &mut dictionary,
        "selected_arm",
        attempt
            .selected_arm
            .map(|arm| (arm as i64).to_variant())
            .unwrap_or_else(Variant::nil),
    );
    dictionary.set("outcome", plural_outcome_name(&attempt.outcome));
    dictionary.to_variant()
}

fn plural_outcome_name(outcome: &PluralResolutionOutcome) -> &'static str {
    match outcome {
        PluralResolutionOutcome::MissingPluralForms => "missing_plural_forms",
        PluralResolutionOutcome::MissingEntry => "missing_entry",
        PluralResolutionOutcome::MissingTranslation => "missing_translation",
        PluralResolutionOutcome::Matched => "matched",
    }
}
