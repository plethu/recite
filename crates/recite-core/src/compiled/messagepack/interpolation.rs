use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use super::CompiledAssetDecodeError;
use crate::InterpolationType;
use crate::compiled::CompiledInterpolationBinding;
use crate::compiled::CompiledLine;

#[derive(Deserialize)]
pub(crate) struct MsgInterpolationBinding(pub(crate) String, pub(crate) String, pub(crate) String);

impl TryFrom<MsgInterpolationBinding> for CompiledInterpolationBinding {
    type Error = CompiledAssetDecodeError;

    fn try_from(value: MsgInterpolationBinding) -> Result<Self, Self::Error> {
        let value_type = match value.2.as_str() {
            "string" => InterpolationType::String,
            "int" => InterpolationType::Integer,
            "float" => InterpolationType::Float,
            "bool" => InterpolationType::Boolean,
            other => {
                return Err(CompiledAssetDecodeError::MalformedAsset(format!(
                    "unknown interpolation type `{other}`"
                )));
            }
        };
        Ok(Self {
            name: value.0,
            value: value.1,
            value_type,
        })
    }
}

pub(crate) fn validate_interpolation_row(
    source_text: &str,
    authored_source_text: &str,
    bindings: &[CompiledInterpolationBinding],
) -> Result<(), CompiledAssetDecodeError> {
    validate_interpolation_row_inner(source_text, authored_source_text, bindings, &[])
}

pub(crate) fn validate_line_interpolation_rows(
    line: &CompiledLine,
) -> Result<(), CompiledAssetDecodeError> {
    if line.interpolation_mode == crate::CompiledInterpolationMode::Legacy {
        return Ok(());
    }
    let ignored = if let (Some(_source_text), Some(authored_plural_source_text)) = (
        line.plural_source_text.as_deref(),
        line.authored_plural_source_text.as_deref(),
    ) {
        plural_unused_bindings(
            &line.authored_source_text,
            authored_plural_source_text,
            &line.interpolation_bindings,
        )
    } else {
        Vec::new()
    };
    validate_interpolation_row_inner(
        &line.source_text,
        &line.authored_source_text,
        &line.interpolation_bindings,
        &ignored,
    )?;
    match (
        line.plural_source_text.as_deref(),
        line.authored_plural_source_text.as_deref(),
    ) {
        (None, None) => Ok(()),
        (Some(source_text), Some(authored_source_text)) => {
            if crate::decode_interpolation_text(authored_source_text) != source_text {
                return Err(CompiledAssetDecodeError::MalformedAsset(
                    "compiled plural interpolation source text does not match authored source text"
                        .to_owned(),
                ));
            }
            let ignored = plural_unused_bindings(
                authored_source_text,
                &line.authored_source_text,
                &line.interpolation_bindings,
            );
            validate_interpolation_row_inner(
                source_text,
                authored_source_text,
                &line.interpolation_bindings,
                &ignored,
            )
        }
        _ => Err(CompiledAssetDecodeError::MalformedAsset(
            "compiled plural source text must include both decoded and authored forms".to_owned(),
        )),
    }
}

fn plural_unused_bindings<'a>(
    source_text: &str,
    other_source_text: &str,
    bindings: &'a [CompiledInterpolationBinding],
) -> Vec<&'a str> {
    let Ok(source_names) = crate::extract_placeholder_names(source_text) else {
        return Vec::new();
    };
    let Ok(other_names) = crate::extract_placeholder_names(other_source_text) else {
        return Vec::new();
    };
    bindings
        .iter()
        .filter(|binding| {
            !source_names.contains(&binding.name)
                && (binding.name == "count" || other_names.contains(&binding.name))
        })
        .map(|binding| binding.name.as_str())
        .collect()
}

fn validate_interpolation_row_inner(
    source_text: &str,
    authored_source_text: &str,
    bindings: &[CompiledInterpolationBinding],
    ignored_unused: &[&str],
) -> Result<(), CompiledAssetDecodeError> {
    if crate::decode_interpolation_text(authored_source_text) != source_text {
        return Err(CompiledAssetDecodeError::MalformedAsset(
            "compiled interpolation source text does not match authored source text".to_owned(),
        ));
    }
    let occurrences =
        crate::extract_placeholder_occurrences(authored_source_text).map_err(|error| {
            CompiledAssetDecodeError::MalformedAsset(format!(
                "invalid authored interpolation source text: {}",
                error.message()
            ))
        })?;
    let mut declared = BTreeSet::new();
    for binding in bindings {
        if !is_interpolation_name(&binding.name) {
            return Err(CompiledAssetDecodeError::MalformedAsset(format!(
                "invalid interpolation binding name `{}`",
                binding.name
            )));
        }
        if !is_interpolation_name(&binding.value) {
            return Err(CompiledAssetDecodeError::MalformedAsset(format!(
                "invalid interpolation value name `{}`",
                binding.value
            )));
        }
        if !declared.insert(binding.name.as_str()) {
            return Err(CompiledAssetDecodeError::MalformedAsset(format!(
                "interpolation binding `{}` appears more than once",
                binding.name
            )));
        }
    }
    let occurrence_counts =
        occurrences
            .iter()
            .fold(BTreeMap::<&str, usize>::new(), |mut counts, name| {
                *counts.entry(name.as_str()).or_default() += 1;
                counts
            });
    for name in occurrence_counts.keys() {
        if !declared.contains(name) {
            return Err(CompiledAssetDecodeError::MalformedAsset(format!(
                "placeholder `{name}` has no interpolation binding"
            )));
        }
    }
    for name in declared {
        if !occurrence_counts.contains_key(name) && !ignored_unused.contains(&name) {
            return Err(CompiledAssetDecodeError::MalformedAsset(format!(
                "interpolation binding `{name}` is not used in authored source text"
            )));
        }
    }
    Ok(())
}

fn is_interpolation_name(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some(character) if character.is_ascii_lowercase())
        && chars.all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}
