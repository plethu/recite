use super::Field;
use recite_runtime::{
    LocalizedLookupTrace, PluralLineTrace,
    localisation::{LocaleLookupOutcome, PluralResolutionOutcome, TextDomain},
};

pub(super) fn singular(lookup: &LocalizedLookupTrace, index: usize) -> Option<Field<'_>> {
    Some(match index {
        0 => Field::text("Stable ID", lookup.id.as_str()),
        1 => Field::text("Source text", lookup.source_text.as_str()),
        2 => Field::optional("Resolved text", lookup.resolved_text.as_deref()),
        3 => Field::text(
            "Text domain",
            match lookup.domain {
                TextDomain::Line => "Line",
                TextDomain::Choice => "Choice",
                TextDomain::PresentationLabel => "Presentation label",
                TextDomain::AvailabilityReason => "Availability reason",
            },
        ),
        4 => Field::optional("Matched language", lookup.matched_locale.as_deref()),
        5 => Field::optional("Matched context", lookup.matched_context.as_deref()),
        6 => Field::optional("Matched key", lookup.matched_key.as_deref()),
        7 => Field::text("Outcome", outcome(&lookup.outcome)),
        8 => Field::display("Lookup attempts", lookup.attempts.len()),
        index => {
            let index = index - 9;
            let attempt = lookup.attempts.get(index / 4)?;
            match index % 4 {
                0 => Field::text(
                    format!("Attempt {} · Language", index / 4 + 1),
                    attempt.locale.as_str(),
                ),
                1 => Field::text(
                    format!("Attempt {} · Context", index / 4 + 1),
                    attempt.context.as_str(),
                ),
                2 => Field::text(
                    format!("Attempt {} · Key", index / 4 + 1),
                    attempt.key.as_str(),
                ),
                _ => Field::text(
                    format!("Attempt {} · Outcome", index / 4 + 1),
                    outcome(&attempt.outcome),
                ),
            }
        }
    })
}
fn outcome(outcome: &LocaleLookupOutcome) -> &'static str {
    match outcome {
        LocaleLookupOutcome::Matched => "Matched",
        LocaleLookupOutcome::MissingEntry => "Missing entry",
    }
}
pub(super) fn plural<'a>(
    id: &'a str,
    lookup: &'a PluralLineTrace,
    index: usize,
) -> Option<Field<'a>> {
    Some(match index {
        0 => Field::text("Stable ID", id),
        1 => Field::text("Singular source text", lookup.singular_source_text.as_str()),
        2 => Field::text("Plural source text", lookup.plural_source_text.as_str()),
        3 => Field::display("Count", lookup.count),
        4 => Field::display("Selected plural form", lookup.selected_arm + 1),
        5 => Field::optional("Matched language", lookup.matched_locale.as_deref()),
        6 => Field::optional("Matched context", lookup.matched_context.as_deref()),
        7 => Field::optional("Matched key", lookup.matched_key.as_deref()),
        8 => Field::text(
            "Matched plural form",
            lookup
                .matched_arm
                .map(|arm| (arm + 1).to_string())
                .unwrap_or_else(|| "None".into()),
        ),
        9 => Field::text(
            "Source fallback form",
            lookup
                .source_fallback_arm
                .map(|arm| (arm + 1).to_string())
                .unwrap_or_else(|| "None".into()),
        ),
        10 => Field::display("Lookup attempts", lookup.attempts.len()),
        index => {
            let index = index - 11;
            let attempt = lookup.attempts.get(index / 5)?;
            match index % 5 {
                0 => Field::text(
                    format!("Attempt {} · Language", index / 5 + 1),
                    attempt.locale.as_str(),
                ),
                1 => Field::text(
                    format!("Attempt {} · Context", index / 5 + 1),
                    attempt.context.as_str(),
                ),
                2 => Field::text(
                    format!("Attempt {} · Key", index / 5 + 1),
                    attempt.key.as_str(),
                ),
                3 => Field::text(
                    format!("Attempt {} · Plural form", index / 5 + 1),
                    attempt
                        .selected_arm
                        .map(|arm| (arm + 1).to_string())
                        .unwrap_or_else(|| "None".into()),
                ),
                _ => Field::text(
                    format!("Attempt {} · Outcome", index / 5 + 1),
                    match attempt.outcome {
                        PluralResolutionOutcome::Matched => "Matched",
                        PluralResolutionOutcome::MissingPluralForms => "Missing plural metadata",
                        PluralResolutionOutcome::MissingEntry => "Missing entry",
                        PluralResolutionOutcome::MissingTranslation => "Missing translation",
                    },
                ),
            }
        }
    })
}
