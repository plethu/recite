use super::{CompiledAssetDecodeError, malformed};
use crate::compiled::{CompiledDialogue, MetadataIndex, StatementIndex};

mod semantics;
mod tables;
use semantics::{
    validate_choice_echo, validate_condition, validate_divert, validate_effect, validate_non_empty,
    validate_reason_value, validate_span, validate_statement, validate_value,
};
use tables::{
    ensure_availability_reason, ensure_index, ensure_range, ensure_unique_strings,
    validate_disjoint_ids, validate_lookup_entries,
};

pub(super) fn validate_dialogue(
    dialogue: &CompiledDialogue,
) -> Result<(), CompiledAssetDecodeError> {
    let block_len = dialogue.blocks.len();
    ensure_index("default block", block_len, dialogue.default_block.as_u32())?;

    for (index, source) in dialogue.sources.iter().enumerate() {
        if source.path.is_empty() {
            return Err(malformed(format!(
                "source file {index} path must not be empty"
            )));
        }
    }
    ensure_unique_strings(
        "source file path",
        dialogue.sources.iter().map(|source| source.path.as_str()),
    )?;

    for block in &dialogue.blocks {
        ensure_index(
            "block source file",
            dialogue.sources.len(),
            block.source_file.as_u32(),
        )?;
        ensure_range(
            "block statements",
            dialogue.statements.len(),
            block.statements,
            StatementIndex::as_u32,
        )?;
        ensure_range(
            "block metadata",
            dialogue.metadata.len(),
            block.metadata,
            MetadataIndex::as_u32,
        )?;
        if let Some(speaker) = block.default_speaker {
            ensure_index(
                "block default speaker",
                dialogue.speakers.len(),
                speaker.as_u32(),
            )?;
        }
        ensure_index(
            "block source map",
            dialogue.source_maps.len(),
            block.source_map.as_u32(),
        )?;
    }

    for statement in &dialogue.statements {
        validate_statement(dialogue, statement)?;
    }
    for arm in &dialogue.match_arms {
        ensure_range(
            "match arm statements",
            dialogue.statements.len(),
            arm.statements,
            StatementIndex::as_u32,
        )?;
        ensure_index(
            "match arm source map",
            dialogue.source_maps.len(),
            arm.source_map.as_u32(),
        )?;
    }
    for line in &dialogue.lines {
        super::interpolation::validate_line_interpolation_rows(line)?;
        if line.plural_source_text.is_some()
            && !line.interpolation_bindings.iter().any(|binding| {
                binding.name == "count" && binding.value_type == crate::InterpolationType::Integer
            })
        {
            return Err(malformed(
                "plural line requires an integer `count` interpolation binding".to_owned(),
            ));
        }
        if line.plural_source_text.is_none() && line.authored_plural_source_text.is_some() {
            return Err(malformed(
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
    for choice in &dialogue.choices {
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
    for metadata in &dialogue.metadata {
        validate_non_empty("metadata key", &metadata.key)?;
        validate_value(&metadata.value)?;
        if let Some(source_map) = metadata.source_map {
            ensure_index(
                "metadata source map",
                dialogue.source_maps.len(),
                source_map.as_u32(),
            )?;
        }
    }
    for effect in &dialogue.effects {
        ensure_index(
            "effect source map",
            dialogue.source_maps.len(),
            effect.source_map.as_u32(),
        )?;
        validate_effect(&effect.function, &effect.args)?;
    }
    ensure_unique_strings(
        "effect id",
        dialogue.effects.iter().map(|effect| effect.id.as_str()),
    )?;
    ensure_unique_strings(
        "availability reason id",
        dialogue
            .availability_reasons
            .iter()
            .map(|reason| reason.id.as_str()),
    )?;
    ensure_unique_strings(
        "condition availability reason function",
        dialogue
            .condition_availability_reasons
            .iter()
            .map(|mapping| mapping.function.as_str()),
    )?;
    for mapping in &dialogue.condition_availability_reasons {
        validate_non_empty("condition availability reason function", &mapping.function)?;
        ensure_availability_reason(
            dialogue,
            "condition availability reason mapping",
            mapping.reason.as_str(),
        )?;
        for argument in &mapping.args {
            validate_non_empty("availability reason argument name", &argument.name)?;
            validate_reason_value(&argument.value)?;
        }
    }
    for source_map in &dialogue.source_maps {
        validate_span(&source_map.span)?;
        ensure_index(
            "source map source file",
            dialogue.sources.len(),
            source_map.source_file.as_u32(),
        )?;
        let source = &dialogue.sources[source_map.source_file.as_u32() as usize];
        if source.path != source_map.span.file {
            return Err(malformed(format!(
                "source map span file `{}` does not match source file `{}`",
                source_map.span.file, source.path
            )));
        }
    }
    validate_lookup_entries(
        "block lookup",
        dialogue
            .blocks
            .iter()
            .map(|block| block.id.as_str())
            .collect(),
        dialogue
            .block_lookup
            .as_slice()
            .iter()
            .map(|entry| (entry.id.as_str(), entry.index.as_u32())),
    )?;
    validate_lookup_entries(
        "line lookup",
        dialogue.lines.iter().map(|line| line.id.as_str()).collect(),
        dialogue
            .line_lookup
            .as_slice()
            .iter()
            .map(|entry| (entry.id.as_str(), entry.index.as_u32())),
    )?;
    validate_lookup_entries(
        "choice lookup",
        dialogue
            .choices
            .iter()
            .map(|choice| choice.id.as_str())
            .collect(),
        dialogue
            .choice_lookup
            .as_slice()
            .iter()
            .map(|entry| (entry.id.as_str(), entry.index.as_u32())),
    )?;
    validate_disjoint_ids(
        "line and choice ids",
        dialogue.lines.iter().map(|line| line.id.as_str()),
        dialogue.choices.iter().map(|choice| choice.id.as_str()),
    )?;

    Ok(())
}
