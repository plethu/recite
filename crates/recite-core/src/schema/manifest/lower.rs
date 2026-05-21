use std::collections::BTreeSet;

use crate::Diagnostic;

use crate::schema::{
    ConditionDefinition, ConditionReturnType, EffectDefinition, EnumTypeDefinition,
    MarkupDefinition, MetadataDefinition, ParameterDefinition, ProjectSchema, RegistryDefinition,
    SchemaTypeDefinition, SchemaTypeRef, SpeakerDefinition,
};

use super::SchemaLoadReport;
use super::diagnostics::{
    DUPLICATE_DEFINITION, INVALID_TYPE_REFERENCE, MALFORMED_SHAPE, UNSUPPORTED_VERSION, diagnostic,
};
use super::raw::{
    Named, RawConditionDefinition, RawEffectDefinition, RawManifest, RawMarkupDefinition,
    RawMetadataDefinition, RawParameterDefinition, RawRegistryDefinition, RawSpeakerDefinition,
    RawTypeDefinition,
};
use super::spans::{ManifestSpans, top_level_key_span, top_level_number_token};
use super::validate::{
    PendingTypeReference, duplicate_definition, parse_effect_mode, parse_enum_return,
    parse_metadata_target, parse_type_ref, validate_manifest_name, validate_non_empty_string,
    validate_type_references,
};

pub(crate) fn lower_manifest(file: String, source: &str, raw: RawManifest) -> SchemaLoadReport {
    let mut diagnostics = Vec::new();
    let mut schema = ProjectSchema::empty_v1();
    let mut spans = ManifestSpans::new();
    let mut pending_type_refs = Vec::new();

    match schema_version(source, &raw.schema_version) {
        SchemaVersion::One => {}
        SchemaVersion::Unsupported(version) => diagnostics.push(diagnostic(
            UNSUPPORTED_VERSION,
            format!("unsupported schema manifest version {version}"),
            top_level_key_span(&file, source, "schema_version"),
        )),
        SchemaVersion::Malformed => diagnostics.push(diagnostic(
            MALFORMED_SHAPE,
            "schema_version must be an integer",
            top_level_key_span(&file, source, "schema_version"),
        )),
    }

    spans.enter_section(source, "types");
    lower_types(
        &file,
        source,
        &mut spans,
        raw.types,
        &mut schema,
        &mut diagnostics,
    );
    spans.enter_section(source, "registries");
    lower_registries(
        &file,
        source,
        &mut spans,
        raw.registries,
        &mut schema,
        &mut diagnostics,
    );
    spans.enter_section(source, "speakers");
    lower_speakers(
        &file,
        source,
        &mut spans,
        raw.speakers,
        &mut schema,
        &mut diagnostics,
    );
    spans.enter_section(source, "conditions");
    lower_conditions(
        &file,
        source,
        &mut spans,
        raw.conditions,
        &mut schema,
        &mut diagnostics,
        &mut pending_type_refs,
    );
    spans.enter_section(source, "effects");
    lower_effects(
        &file,
        source,
        &mut spans,
        raw.effects,
        &mut schema,
        &mut diagnostics,
        &mut pending_type_refs,
    );
    spans.enter_section(source, "metadata");
    lower_metadata(
        &file,
        source,
        &mut spans,
        raw.metadata,
        &mut schema,
        &mut diagnostics,
        &mut pending_type_refs,
    );
    spans.enter_section(source, "markup");
    lower_markup(
        &file,
        source,
        &mut spans,
        raw.markup,
        &mut schema,
        &mut diagnostics,
    );
    validate_type_references(&schema, &pending_type_refs, &mut diagnostics);

    let schema = diagnostics.is_empty().then_some(schema);
    SchemaLoadReport {
        schema,
        diagnostics,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SchemaVersion<'a> {
    One,
    Unsupported(&'a str),
    Malformed,
}

fn schema_version<'a>(source: &'a str, value: &serde_json::Value) -> SchemaVersion<'a> {
    if !value.is_number() {
        return SchemaVersion::Malformed;
    }

    let Some(token) = top_level_number_token(source, "schema_version") else {
        return SchemaVersion::Malformed;
    };

    if number_token_equals_one(token) {
        SchemaVersion::One
    } else {
        SchemaVersion::Unsupported(token)
    }
}

fn number_token_equals_one(token: &str) -> bool {
    let Some((significand, exponent)) = split_decimal_exponent(token) else {
        return false;
    };
    if significand.starts_with('-') {
        return false;
    }

    let Some((integer, fraction)) = significand.split_once('.').or(Some((significand, ""))) else {
        return false;
    };
    let coefficient = format!("{integer}{fraction}");
    if coefficient.is_empty()
        || !coefficient
            .chars()
            .all(|character| character.is_ascii_digit())
    {
        return false;
    }

    let coefficient = coefficient.trim_start_matches('0');
    if coefficient.is_empty() {
        return false;
    }

    let decimal_places = i64::try_from(fraction.len()).unwrap_or(i64::MAX);
    let scale = decimal_places - exponent;
    if scale < 0 {
        return false;
    };
    let Ok(scale) = usize::try_from(scale) else {
        return false;
    };

    let Some(expected_len) = scale.checked_add(1) else {
        return false;
    };
    let mut bytes = coefficient.bytes();
    coefficient.len() == expected_len
        && bytes.next() == Some(b'1')
        && bytes.all(|byte| byte == b'0')
}

fn split_decimal_exponent(token: &str) -> Option<(&str, i64)> {
    let Some(index) = token.find(['e', 'E']) else {
        return Some((token, 0));
    };
    let significand = &token[..index];
    let exponent = token[index + 1..].parse::<i64>().ok()?;
    Some((significand, exponent))
}

fn lower_types(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    entries: Vec<Named<RawTypeDefinition>>,
    schema: &mut ProjectSchema,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut seen = BTreeSet::new();
    for entry in entries {
        let name_span = spans.next_key_span(file, source, &entry.name);
        if !validate_manifest_name(diagnostics, "type name", &entry.name, name_span.clone()) {
            continue;
        }
        if !seen.insert(entry.name.clone()) {
            duplicate_definition(diagnostics, "type", &entry.name, name_span);
            continue;
        }

        let kind_span = spans.next_value_span(file, source, &entry.value.kind);
        if entry.value.kind != "enum" {
            diagnostics.push(diagnostic(
                MALFORMED_SHAPE,
                format!(
                    "type '{}' uses unsupported kind '{}'",
                    entry.name, entry.value.kind
                ),
                kind_span,
            ));
            continue;
        }

        let values = canonical_string_values(
            file,
            source,
            spans,
            diagnostics,
            &format!("enum '{}'", entry.name),
            &entry.value.values,
        );
        schema.types.insert(
            entry.name,
            SchemaTypeDefinition::Enum(EnumTypeDefinition { values }),
        );
    }
}

fn lower_registries(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    entries: Vec<Named<RawRegistryDefinition>>,
    schema: &mut ProjectSchema,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut seen = BTreeSet::new();
    for entry in entries {
        let name_span = spans.next_key_span(file, source, &entry.name);
        if !validate_manifest_name(diagnostics, "registry name", &entry.name, name_span.clone()) {
            continue;
        }
        if !seen.insert(entry.name.clone()) {
            duplicate_definition(diagnostics, "registry", &entry.name, name_span);
            continue;
        }

        let values = canonical_string_values(
            file,
            source,
            spans,
            diagnostics,
            &format!("registry '{}'", entry.name),
            &entry.value.values,
        );
        if let Some(origin) = &entry.value.origin {
            let origin_span = spans.next_value_span(file, source, origin);
            validate_non_empty_string(diagnostics, "registry origin", origin, origin_span);
        }
        schema.registries.insert(
            entry.name,
            RegistryDefinition {
                values,
                origin: entry.value.origin,
            },
        );
    }
}

fn lower_speakers(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    entries: Vec<Named<RawSpeakerDefinition>>,
    schema: &mut ProjectSchema,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut seen = BTreeSet::new();
    for entry in entries {
        let name_span = spans.next_key_span(file, source, &entry.name);
        if !validate_manifest_name(diagnostics, "speaker name", &entry.name, name_span.clone()) {
            continue;
        }
        if !seen.insert(entry.name.clone()) {
            duplicate_definition(diagnostics, "speaker", &entry.name, name_span);
            continue;
        }

        if let Some(display_name) = &entry.value.display_name {
            let display_name_span = spans.next_value_span(file, source, display_name);
            validate_non_empty_string(
                diagnostics,
                "speaker display_name",
                display_name,
                display_name_span,
            );
        }
        schema.speakers.insert(
            entry.name,
            SpeakerDefinition {
                display_name: entry.value.display_name,
            },
        );
    }
}

fn lower_conditions(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    entries: Vec<Named<RawConditionDefinition>>,
    schema: &mut ProjectSchema,
    diagnostics: &mut Vec<Diagnostic>,
    pending_type_refs: &mut Vec<PendingTypeReference>,
) {
    let mut seen = BTreeSet::new();
    for entry in entries {
        let name_span = spans.next_key_span(file, source, &entry.name);
        if !validate_manifest_name(
            diagnostics,
            "condition name",
            &entry.name,
            name_span.clone(),
        ) {
            continue;
        }
        if !seen.insert(entry.name.clone()) {
            duplicate_definition(diagnostics, "condition", &entry.name, name_span);
            continue;
        }

        let params = lower_params(
            file,
            source,
            spans,
            diagnostics,
            &format!("condition '{}'", entry.name),
            &entry.value.params,
            pending_type_refs,
        );
        let returns = match entry.value.returns.as_deref() {
            None | Some("bool") => ConditionReturnType::Bool,
            Some(value) => {
                let return_span = spans.next_value_span(file, source, value);
                match parse_enum_return(value) {
                    Some(name) => {
                        pending_type_refs.push(PendingTypeReference {
                            owner: format!("condition '{}' return type", entry.name),
                            type_ref: SchemaTypeRef::Enum(name.clone()),
                            span: return_span,
                        });
                        ConditionReturnType::Enum(name)
                    }
                    None => {
                        diagnostics.push(diagnostic(
                            INVALID_TYPE_REFERENCE,
                            format!(
                                "condition '{}' has invalid return type '{}'",
                                entry.name, value
                            ),
                            return_span,
                        ));
                        ConditionReturnType::Bool
                    }
                }
            }
        };

        schema
            .conditions
            .insert(entry.name, ConditionDefinition { params, returns });
    }
}

fn lower_effects(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    entries: Vec<Named<RawEffectDefinition>>,
    schema: &mut ProjectSchema,
    diagnostics: &mut Vec<Diagnostic>,
    pending_type_refs: &mut Vec<PendingTypeReference>,
) {
    let mut seen = BTreeSet::new();
    for entry in entries {
        let name_span = spans.next_key_span(file, source, &entry.name);
        if !validate_manifest_name(diagnostics, "effect name", &entry.name, name_span.clone()) {
            continue;
        }
        if !seen.insert(entry.name.clone()) {
            duplicate_definition(diagnostics, "effect", &entry.name, name_span);
            continue;
        }

        let mut modes = BTreeSet::new();
        for mode in &entry.value.modes {
            let mode_span = spans.next_value_span(file, source, mode);
            let Some(effect_mode) = parse_effect_mode(mode) else {
                diagnostics.push(diagnostic(
                    MALFORMED_SHAPE,
                    format!("effect '{}' uses unsupported mode '{}'", entry.name, mode),
                    mode_span,
                ));
                continue;
            };

            if !modes.insert(effect_mode) {
                diagnostics.push(diagnostic(
                    DUPLICATE_DEFINITION,
                    format!("effect '{}' repeats mode '{}'", entry.name, mode),
                    mode_span,
                ));
            }
        }

        let params = lower_params(
            file,
            source,
            spans,
            diagnostics,
            &format!("effect '{}'", entry.name),
            &entry.value.params,
            pending_type_refs,
        );
        schema
            .effects
            .insert(entry.name, EffectDefinition { modes, params });
    }
}

fn lower_metadata(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    entries: Vec<Named<RawMetadataDefinition>>,
    schema: &mut ProjectSchema,
    diagnostics: &mut Vec<Diagnostic>,
    pending_type_refs: &mut Vec<PendingTypeReference>,
) {
    let mut seen = BTreeSet::new();
    for entry in entries {
        let name_span = spans.next_key_span(file, source, &entry.name);
        if !validate_manifest_name(diagnostics, "metadata name", &entry.name, name_span.clone()) {
            continue;
        }
        if !seen.insert(entry.name.clone()) {
            duplicate_definition(diagnostics, "metadata", &entry.name, name_span);
            continue;
        }

        let mut targets = BTreeSet::new();
        for target in &entry.value.targets {
            let target_span = spans.next_value_span(file, source, target);
            let Some(metadata_target) = parse_metadata_target(target) else {
                diagnostics.push(diagnostic(
                    MALFORMED_SHAPE,
                    format!(
                        "metadata '{}' uses unsupported target '{}'",
                        entry.name, target
                    ),
                    target_span,
                ));
                continue;
            };

            if !targets.insert(metadata_target) {
                diagnostics.push(diagnostic(
                    DUPLICATE_DEFINITION,
                    format!("metadata '{}' repeats target '{}'", entry.name, target),
                    target_span,
                ));
            }
        }

        let type_ref_span = spans.next_value_span(file, source, &entry.value.type_ref);
        let type_ref = parse_type_ref(&entry.value.type_ref).unwrap_or_else(|| {
            diagnostics.push(diagnostic(
                INVALID_TYPE_REFERENCE,
                format!(
                    "metadata '{}' has invalid type reference '{}'",
                    entry.name, entry.value.type_ref
                ),
                type_ref_span.clone(),
            ));
            SchemaTypeRef::String
        });
        if parse_type_ref(&entry.value.type_ref).is_some() {
            pending_type_refs.push(PendingTypeReference {
                owner: format!("metadata '{}'", entry.name),
                type_ref: type_ref.clone(),
                span: type_ref_span,
            });
        }

        schema.metadata.insert(
            entry.name,
            MetadataDefinition {
                targets,
                type_ref,
                repeatable: entry.value.repeatable,
            },
        );
    }
}

fn lower_markup(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    entries: Vec<Named<RawMarkupDefinition>>,
    schema: &mut ProjectSchema,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut seen = BTreeSet::new();
    for entry in entries {
        let name_span = spans.next_key_span(file, source, &entry.name);
        if !validate_manifest_name(diagnostics, "markup name", &entry.name, name_span.clone()) {
            continue;
        }
        if !seen.insert(entry.name.clone()) {
            duplicate_definition(diagnostics, "markup", &entry.name, name_span);
            continue;
        }

        schema.markup.insert(
            entry.name,
            MarkupDefinition {
                requires_closing: entry.value.requires_closing,
                translatable: entry.value.translatable,
                allows_nesting: entry.value.allows_nesting.unwrap_or(true),
            },
        );
    }
}

fn lower_params(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    diagnostics: &mut Vec<Diagnostic>,
    owner: &str,
    params: &[RawParameterDefinition],
    pending_type_refs: &mut Vec<PendingTypeReference>,
) -> Vec<ParameterDefinition> {
    let mut seen = BTreeSet::new();
    params
        .iter()
        .map(|param| {
            let name_span = spans.next_value_span(file, source, &param.name);
            if validate_non_empty_string(
                diagnostics,
                "parameter name",
                &param.name,
                name_span.clone(),
            ) {
                validate_manifest_name(
                    diagnostics,
                    "parameter name",
                    &param.name,
                    name_span.clone(),
                );
            }
            if !seen.insert(param.name.clone()) {
                diagnostics.push(diagnostic(
                    DUPLICATE_DEFINITION,
                    format!("{owner} repeats parameter '{}'", param.name),
                    name_span,
                ));
            }

            let type_ref_span = spans.next_value_span(file, source, &param.type_ref);
            let type_ref = parse_type_ref(&param.type_ref).unwrap_or_else(|| {
                diagnostics.push(diagnostic(
                    INVALID_TYPE_REFERENCE,
                    format!(
                        "parameter '{}' has invalid type reference '{}'",
                        param.name, param.type_ref
                    ),
                    type_ref_span.clone(),
                ));
                SchemaTypeRef::String
            });
            if parse_type_ref(&param.type_ref).is_some() {
                pending_type_refs.push(PendingTypeReference {
                    owner: format!("{owner} parameter '{}'", param.name),
                    type_ref: type_ref.clone(),
                    span: type_ref_span,
                });
            }

            ParameterDefinition {
                name: param.name.clone(),
                type_ref,
            }
        })
        .collect()
}

fn canonical_string_values(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    diagnostics: &mut Vec<Diagnostic>,
    owner: &str,
    values: &[String],
) -> BTreeSet<String> {
    let mut canonical = BTreeSet::new();
    for value in values {
        let value_span = spans.next_value_span(file, source, value);
        if !validate_non_empty_string(diagnostics, "schema value", value, value_span.clone()) {
            continue;
        }
        if !canonical.insert(value.clone()) {
            diagnostics.push(diagnostic(
                DUPLICATE_DEFINITION,
                format!("{owner} repeats value '{value}'"),
                value_span,
            ));
        }
    }
    canonical
}
