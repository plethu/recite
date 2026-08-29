use crate::ScalarValue;

use super::{CompiledAssetDecodeError, malformed};
use crate::compiled::{
    ChoiceIndex, CompiledArgument, CompiledChoiceEcho, CompiledConditionExpression,
    CompiledDialogue, CompiledDivertTarget, CompiledStatement, CompiledStatementKind,
    MatchArmIndex, MetadataIndex, StatementIndex,
};

mod tables;
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
        ensure_availability_reason(
            dialogue,
            "condition availability reason mapping",
            mapping.reason.as_str(),
        )?;
    }
    for source_map in &dialogue.source_maps {
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

fn validate_statement(
    dialogue: &CompiledDialogue,
    statement: &CompiledStatement,
) -> Result<(), CompiledAssetDecodeError> {
    match &statement.kind {
        CompiledStatementKind::Line(index) => {
            ensure_index("line statement", dialogue.lines.len(), index.as_u32())?;
        }
        CompiledStatementKind::Prompt { line, choices } => {
            if let Some(index) = line {
                ensure_index("prompt line", dialogue.lines.len(), index.as_u32())?;
            }
            if choices.len == 0 {
                return Err(malformed("prompt choices must not be empty".to_owned()));
            }
            ensure_range(
                "prompt choices",
                dialogue.choices.len(),
                *choices,
                ChoiceIndex::as_u32,
            )?;
        }
        CompiledStatementKind::Divert(target) => validate_divert(dialogue, target)?,
        CompiledStatementKind::If {
            condition,
            then_statements,
            else_statements,
        } => {
            validate_condition(condition)?;
            ensure_range(
                "if then statements",
                dialogue.statements.len(),
                *then_statements,
                StatementIndex::as_u32,
            )?;
            ensure_range(
                "if else statements",
                dialogue.statements.len(),
                *else_statements,
                StatementIndex::as_u32,
            )?;
        }
        CompiledStatementKind::Match { scrutinee, arms } => {
            for argument in &scrutinee.args {
                validate_argument(argument)?;
            }
            ensure_range(
                "match arms",
                dialogue.match_arms.len(),
                *arms,
                MatchArmIndex::as_u32,
            )?;
        }
        CompiledStatementKind::Effect(index) => {
            ensure_index("effect statement", dialogue.effects.len(), index.as_u32())?;
        }
        CompiledStatementKind::End => {}
    }
    ensure_index(
        "statement source map",
        dialogue.source_maps.len(),
        statement.source_map.as_u32(),
    )?;
    Ok(())
}

fn validate_divert(
    dialogue: &CompiledDialogue,
    target: &CompiledDivertTarget,
) -> Result<(), CompiledAssetDecodeError> {
    if let CompiledDivertTarget::Block(index) = target {
        ensure_index("block divert", dialogue.blocks.len(), index.as_u32())?;
    }
    Ok(())
}

fn validate_condition(
    condition: &CompiledConditionExpression,
) -> Result<(), CompiledAssetDecodeError> {
    match condition {
        CompiledConditionExpression::Call(call) => {
            for argument in &call.args {
                validate_argument(argument)?;
            }
        }
        CompiledConditionExpression::And(expressions) => {
            if expressions.is_empty() {
                return Err(malformed(
                    "condition and group must not be empty".to_owned(),
                ));
            }
            for expression in expressions {
                validate_condition(expression)?;
            }
        }
        CompiledConditionExpression::Or(expressions) => {
            if expressions.is_empty() {
                return Err(malformed("condition or group must not be empty".to_owned()));
            }
            for expression in expressions {
                validate_condition(expression)?;
            }
        }
        CompiledConditionExpression::Not(expression) => validate_condition(expression)?,
    }
    Ok(())
}

fn validate_argument(argument: &CompiledArgument) -> Result<(), CompiledAssetDecodeError> {
    if let CompiledArgument::Value(ScalarValue::Float(value)) = argument
        && !value.is_finite()
    {
        return Err(malformed("float scalar must be finite".to_owned()));
    }
    Ok(())
}

fn validate_choice_echo(
    dialogue: &CompiledDialogue,
    echo: &CompiledChoiceEcho,
) -> Result<(), CompiledAssetDecodeError> {
    let CompiledChoiceEcho::ExplicitLine(line_id) = echo else {
        return Ok(());
    };
    if dialogue
        .line_lookup
        .as_slice()
        .binary_search_by(|entry| entry.id.as_str().cmp(line_id.as_str()))
        .is_ok()
    {
        return Ok(());
    }
    Err(malformed(format!(
        "choice echo references unknown line id `{line_id}`"
    )))
}
