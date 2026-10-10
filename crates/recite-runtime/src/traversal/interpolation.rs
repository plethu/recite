use recite_core::{
    ScalarValue,
    ast::InterpolationType,
    compiled::{CompiledInterpolationBinding, CompiledInterpolationMode},
};

use super::output::LocaleLookup;
use super::trace::PluralLineTrace;
use crate::DialogueError;
use crate::event::{DialoguePlural, DialoguePluralResolution, DialoguePluralResolutionOutcome};
use crate::locale::{InterpolationValueProvider, TextDomain};

mod render;

use render::{ResolvedCount, render_selected_bound_template, value_kind};
pub(super) use render::{render_bound_template, render_template};

pub(super) fn localise_text(
    id: &str,
    authored_source_text: &str,
    domain: TextDomain,
    bindings: &[CompiledInterpolationBinding],
    mode: CompiledInterpolationMode,
    locale: LocaleLookup<'_>,
) -> Result<String, DialogueError> {
    let template = if let Some((locale_id, provider)) = locale.locale.zip(locale.provider) {
        let resolved = provider
            .lookup_with_provenance(id, authored_source_text, domain, locale_id, locale.variant)
            .map_err(|error| DialogueError::LocaleLookupFailed {
                id: id.to_owned(),
                reason: error.reason().to_owned(),
            })?;
        if let Some(trace) = locale.trace {
            trace.record_localized_lookup(id, authored_source_text, domain, &resolved);
        }
        if let Some(template) = resolved.template.as_deref()
            && mode == CompiledInterpolationMode::Current
        {
            recite_core::validate_translation_placeholders(authored_source_text, template)
                .map_err(|error| DialogueError::InvalidInterpolationSyntax {
                    reason: error.message(),
                })?;
        }
        resolved.template
    } else {
        None
    };
    match mode {
        CompiledInterpolationMode::Legacy => {
            Ok(template.unwrap_or_else(|| authored_source_text.to_owned()))
        }
        CompiledInterpolationMode::Current => render_bound_template(
            template.as_deref().unwrap_or(authored_source_text),
            bindings,
            locale.values,
        ),
    }
}

pub(super) fn localise_plural_text(
    id: &str,
    source: PluralSource<'_>,
    bindings: &[CompiledInterpolationBinding],
    mode: CompiledInterpolationMode,
    locale: LocaleLookup<'_>,
) -> Result<(String, String, DialoguePlural), DialogueError> {
    let count_binding = bindings
        .iter()
        .find(|binding| binding.name == "count")
        .ok_or_else(|| DialogueError::InvalidPluralCount {
            name: "count".to_owned(),
            reason: "plural line has no count binding".to_owned(),
        })?;
    if count_binding.value_type != InterpolationType::Integer {
        return Err(DialogueError::InvalidPluralCount {
            name: count_binding.value.clone(),
            reason: "expected int binding".to_owned(),
        });
    }
    let count = resolve_count(count_binding, locale.values)?;
    if count < 0 {
        return Err(DialogueError::InvalidPluralCount {
            name: count_binding.value.clone(),
            reason: "count must be non-negative".to_owned(),
        });
    }
    // Source fallback is intentionally English's binary rule. A locale's
    // plural rule is meaningful only when its exact catalogue entry supplies
    // a translation and therefore a selected provider arm.
    let english_arm = usize::from(count != 1);
    let (resolution, translated_arm_count) =
        if let Some((locale_id, provider)) = locale.locale.zip(locale.provider) {
            let resolution = provider
                .resolve_plural(
                    id,
                    source.authored_singular,
                    source.authored_plural,
                    count,
                    TextDomain::Line,
                    locale_id,
                    locale.variant,
                )
                .map_err(|error| DialogueError::LocaleLookupFailed {
                    id: id.to_owned(),
                    reason: error.reason().to_owned(),
                })?;
            let arm_count = if resolution.template.is_some() {
                provider
                    .validated_plural_arm_count(&resolution)
                    .map_err(|error| DialogueError::LocaleLookupFailed {
                        id: id.to_owned(),
                        reason: error.reason().to_owned(),
                    })?
            } else {
                None
            };
            (resolution, arm_count)
        } else {
            (recite_runtime_resolution_without_provider(), None)
        };
    let (selected_arm, source_fallback_arm, matched_arm, outcome) =
        match resolution.template.as_ref() {
            Some(_) => {
                let Some(provider_arm) = resolution.selected_arm else {
                    return Err(DialogueError::LocaleLookupFailed {
                        id: id.to_owned(),
                        reason: "plural provider returned a template without a selected arm"
                            .to_owned(),
                    });
                };
                // Preview prompt projections persist the arm bound, so reject a
                // translated result without evidence before it can be snapshotted.
                if locale.require_plural_arm_count && translated_arm_count.is_none() {
                    return Err(DialogueError::LocaleLookupFailed {
                    id: id.to_owned(),
                    reason:
                        "plural provider returned a translated template without validated arm count"
                            .to_owned(),
                });
                }
                if translated_arm_count
                    .is_some_and(|arm_count| arm_count == 0 || provider_arm >= arm_count)
                {
                    return Err(DialogueError::LocaleLookupFailed {
                        id: id.to_owned(),
                        reason: "plural provider returned an arm outside its validated arm count"
                            .to_owned(),
                    });
                }
                (
                    provider_arm,
                    None,
                    Some(provider_arm),
                    DialoguePluralResolutionOutcome::Translated,
                )
            }
            None => (
                english_arm,
                Some(english_arm),
                None,
                DialoguePluralResolutionOutcome::EnglishSourceFallback,
            ),
        };
    if let Some(trace) = locale.trace {
        match &outcome {
            DialoguePluralResolutionOutcome::Translated => {
                if let Some(arm_count) = translated_arm_count {
                    trace.record_plural_arm_count(id, arm_count);
                }
            }
            DialoguePluralResolutionOutcome::EnglishSourceFallback => {
                trace.record_plural_arm_count(id, 2);
            }
        }
    }
    let source_text = if selected_arm == 0 {
        source.authored_singular
    } else {
        source.authored_plural
    };
    let decoded_source_text = if selected_arm == 0 {
        source.decoded_singular
    } else {
        source.decoded_plural
    };
    let template = resolution
        .template
        .unwrap_or_else(|| source_text.to_owned());
    if mode == CompiledInterpolationMode::Current {
        recite_core::validate_translation_placeholders(source_text, &template).map_err(
            |error| DialogueError::InvalidInterpolationSyntax {
                reason: error.message(),
            },
        )?;
    }
    let rendered = match mode {
        CompiledInterpolationMode::Legacy => template,
        CompiledInterpolationMode::Current => render_selected_bound_template(
            &template,
            bindings,
            locale.values,
            Some(ResolvedCount {
                binding_name: &count_binding.name,
                value_name: &count_binding.value,
                value: count,
            }),
        )?,
    };
    let plural_resolution = DialoguePluralResolution {
        attempts: resolution.attempts.clone(),
        matched_locale: resolution.matched_locale.clone(),
        matched_context: resolution.matched_context.clone(),
        matched_key: resolution.matched_key.clone(),
        matched_arm,
        source_fallback_arm,
        outcome,
    };
    let plural = DialoguePlural {
        singular_source_text: source.authored_singular.to_owned(),
        plural_source_text: source.authored_plural.to_owned(),
        count,
        selected_arm,
        resolution: plural_resolution.clone(),
    };
    if let Some(trace) = locale.trace {
        trace.record_plural_line(
            id,
            PluralLineTrace {
                singular_source_text: source.authored_singular.to_owned(),
                plural_source_text: source.authored_plural.to_owned(),
                count,
                selected_arm,
                attempts: plural_resolution.attempts.clone(),
                matched_locale: plural_resolution.matched_locale.clone(),
                matched_context: plural_resolution.matched_context.clone(),
                matched_key: plural_resolution.matched_key.clone(),
                matched_arm: plural_resolution.matched_arm,
                source_fallback_arm: plural_resolution.source_fallback_arm,
            },
        );
    }
    Ok((rendered, decoded_source_text.to_owned(), plural))
}

fn recite_runtime_resolution_without_provider() -> crate::locale::PluralResolution {
    crate::locale::PluralResolution {
        template: None,
        selected_arm: None,
        matched_locale: None,
        matched_context: None,
        matched_key: None,
        attempts: Vec::new(),
    }
}

pub(super) struct PluralSource<'a> {
    pub(super) authored_singular: &'a str,
    pub(super) authored_plural: &'a str,
    pub(super) decoded_singular: &'a str,
    pub(super) decoded_plural: &'a str,
}

fn resolve_count(
    binding: &CompiledInterpolationBinding,
    values: Option<&dyn InterpolationValueProvider>,
) -> Result<i64, DialogueError> {
    let Some(values) = values else {
        return Err(DialogueError::MissingInterpolationValue {
            name: binding.value.clone(),
        });
    };
    let value = values
        .lookup_value(&binding.value)
        .map_err(|error| DialogueError::InterpolationValueFailed {
            name: binding.value.clone(),
            reason: error.reason().to_owned(),
        })?
        .ok_or_else(|| DialogueError::MissingInterpolationValue {
            name: binding.value.clone(),
        })?;
    match value {
        ScalarValue::Integer(value) => Ok(value),
        value => Err(DialogueError::InvalidPluralCount {
            name: binding.value.clone(),
            reason: format!("expected int, got {}", value_kind(&value)),
        }),
    }
}
