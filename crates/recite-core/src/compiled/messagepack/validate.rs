use std::ops::Range;

use crate::ScalarValue;

use super::{CompiledAssetDecodeError, malformed};
use crate::compiled::{
    ChoiceIndex, CompiledArgument, CompiledChoiceEcho, CompiledConditionExpression,
    CompiledDialogue, CompiledDivertTarget, CompiledStatement, CompiledStatementKind,
    MatchArmIndex, MetadataIndex, StatementIndex, TableRange,
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
        if let Some(condition) = &choice.condition {
            validate_condition(condition)?;
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
fn ensure_unique_strings<'a>(
    field: &'static str,
    values: impl IntoIterator<Item = &'a str>,
) -> Result<(), CompiledAssetDecodeError> {
    let mut values = values.into_iter().collect::<Vec<_>>();
    values.sort_unstable();
    for window in values.windows(2) {
        if window[0] == window[1] {
            return Err(malformed(format!(
                "{field} `{}` appears more than once",
                window[0]
            )));
        }
    }
    Ok(())
}

fn validate_disjoint_ids<'a>(
    field: &'static str,
    left: impl IntoIterator<Item = &'a str>,
    right: impl IntoIterator<Item = &'a str>,
) -> Result<(), CompiledAssetDecodeError> {
    let mut left = left.into_iter().collect::<Vec<_>>();
    let mut right = right.into_iter().collect::<Vec<_>>();
    left.sort_unstable();
    right.sort_unstable();

    let mut left_index = 0;
    let mut right_index = 0;
    while left_index < left.len() && right_index < right.len() {
        match left[left_index].cmp(right[right_index]) {
            std::cmp::Ordering::Less => left_index += 1,
            std::cmp::Ordering::Greater => right_index += 1,
            std::cmp::Ordering::Equal => {
                return Err(malformed(format!(
                    "{field} must be unique, got duplicate `{}`",
                    left[left_index]
                )));
            }
        }
    }
    Ok(())
}

fn validate_lookup_entries<'a>(
    table: &'static str,
    row_ids: Vec<&'a str>,
    entries: impl IntoIterator<Item = (&'a str, u32)>,
) -> Result<(), CompiledAssetDecodeError> {
    let entries = entries.into_iter().collect::<Vec<_>>();
    if entries.len() != row_ids.len() {
        return Err(malformed(format!(
            "{table} has {} entries for {} table rows",
            entries.len(),
            row_ids.len()
        )));
    }

    for (id, index) in entries {
        let Some(row_id) = row_ids.get(index as usize) else {
            return Err(malformed(format!(
                "{table} index {index} is out of range for table length {}",
                row_ids.len()
            )));
        };
        if *row_id != id {
            return Err(malformed(format!(
                "{table} entry `{id}` points to row `{row_id}` at index {index}"
            )));
        }
    }
    Ok(())
}

fn ensure_index(
    field: &'static str,
    table_len: usize,
    index: u32,
) -> Result<(), CompiledAssetDecodeError> {
    if (index as usize) < table_len {
        Ok(())
    } else {
        Err(malformed(format!(
            "{field} index {index} is out of range for table length {table_len}"
        )))
    }
}

fn ensure_range<I: Copy>(
    field: &'static str,
    table_len: usize,
    range: TableRange<I>,
    index: impl Fn(I) -> u32,
) -> Result<Range<usize>, CompiledAssetDecodeError> {
    let start = index(range.start) as usize;
    let len = range.len as usize;
    let end = start
        .checked_add(len)
        .ok_or_else(|| malformed(format!("{field} range overflows usize")))?;

    if end > table_len {
        return Err(malformed(format!(
            "{field} range {start}..{end} exceeds table length {table_len}"
        )));
    }

    Ok(start..end)
}
