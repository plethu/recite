use recite_runtime::{DialoguePlural, DialoguePluralResolution, DialoguePluralResolutionOutcome};

use crate::preview_hash_primitives::hash_optional_u64;
use crate::preview_hash_primitives::{
    hash_i64, hash_len, hash_optional_text, hash_text, hash_u64, tag,
};

pub(super) fn hash_plural(hasher: &mut blake3::Hasher, plural: &DialoguePlural) {
    hash_text(hasher, &plural.singular_source_text);
    hash_text(hasher, &plural.plural_source_text);
    hash_i64(hasher, plural.count);
    hash_u64(hasher, plural.selected_arm as u64);
    hash_plural_resolution(hasher, &plural.resolution);
}

fn hash_plural_resolution(hasher: &mut blake3::Hasher, resolution: &DialoguePluralResolution) {
    hash_len(hasher, resolution.attempts.len());
    for attempt in &resolution.attempts {
        hash_text(hasher, &attempt.locale);
        hash_text(hasher, &attempt.context);
        hash_text(hasher, &attempt.key);
        hash_optional_u64(hasher, attempt.selected_arm.map(|arm| arm as u64));
        tag(
            hasher,
            match attempt.outcome {
                recite_runtime::PluralResolutionOutcome::MissingPluralForms => 0,
                recite_runtime::PluralResolutionOutcome::MissingEntry => 1,
                recite_runtime::PluralResolutionOutcome::MissingTranslation => 2,
                recite_runtime::PluralResolutionOutcome::Matched => 3,
            },
        );
    }
    hash_optional_text(hasher, resolution.matched_locale.as_deref());
    hash_optional_text(hasher, resolution.matched_context.as_deref());
    hash_optional_text(hasher, resolution.matched_key.as_deref());
    hash_optional_u64(hasher, resolution.matched_arm.map(|arm| arm as u64));
    hash_optional_u64(hasher, resolution.source_fallback_arm.map(|arm| arm as u64));
    tag(
        hasher,
        match resolution.outcome {
            DialoguePluralResolutionOutcome::Translated => 0,
            DialoguePluralResolutionOutcome::EnglishSourceFallback => 1,
        },
    );
}
