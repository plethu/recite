use super::super::interpolation;
use super::tables::{ensure_availability_reason, ensure_index, ensure_range};
use super::{ValidationMode, validate_choice_echo, validate_condition, validate_divert};
use crate::compiled::{CompiledDialogue, MetadataIndex};

pub(super) fn validate_lines(
    dialogue: &CompiledDialogue,
    mode: ValidationMode,
) -> Result<(), super::CompiledAssetDecodeError> {
    for line in &dialogue.lines {
        interpolation::validate_line_interpolation_rows(line, mode == ValidationMode::Canonical)?;
        if line.plural_source_text.is_some()
            && !line.interpolation_bindings.iter().any(|binding| {
                binding.name == "count" && binding.value_type == crate::InterpolationType::Integer
            })
        {
            return Err(super::malformed(
                "plural line requires an integer `count` interpolation binding".to_owned(),
            ));
        }
        if line.plural_source_text.is_none() && line.authored_plural_source_text.is_some() {
            return Err(super::malformed(
                "compiled line has an authored plural form without its decoded form".to_owned(),
            ));
        }
        if let Some(speaker) = line.speaker {
            ensure_index("line speaker", dialogue.speakers.len(), speaker.as_u32())?;
        }
        ensure_range(
            "line metadata",
            dialogue.metadata.len(),
            line.metadata,
            MetadataIndex::as_u32,
        )?;
        ensure_index(
            "line source map",
            dialogue.source_maps.len(),
            line.source_map.as_u32(),
        )?;
    }
    Ok(())
}

pub(super) fn validate_choices(
    dialogue: &CompiledDialogue,
    mode: ValidationMode,
) -> Result<(), super::CompiledAssetDecodeError> {
    for choice in &dialogue.choices {
        if choice.interpolation_mode == crate::CompiledInterpolationMode::Current
            || mode == ValidationMode::Canonical
        {
            interpolation::validate_interpolation_row(
                &choice.source_text,
                &choice.authored_source_text,
                &choice.interpolation_bindings,
            )?;
        }
        ensure_range(
            "choice metadata",
            dialogue.metadata.len(),
            choice.metadata,
            MetadataIndex::as_u32,
        )?;
        if let Some(condition) = &choice.availability_requirement {
            validate_condition(condition)?;
        }
        if let Some(reason_id) = &choice.availability_reason_override {
            ensure_availability_reason(
                dialogue,
                "choice availability reason override",
                reason_id.as_str(),
            )?;
        }
        validate_divert(dialogue, &choice.target)?;
        validate_choice_echo(dialogue, &choice.echo)?;
        ensure_index(
            "choice source map",
            dialogue.source_maps.len(),
            choice.source_map.as_u32(),
        )?;
    }
    Ok(())
}
