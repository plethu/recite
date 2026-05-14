use std::ops::Range;

use recite_core::{
    BlockIndex, COMPILED_ASSET_FORMAT_VERSION_V0, COMPILER_COMPATIBILITY_VERSION_V0, ChoiceIndex,
    ChoiceRange, CompiledAssetHeader, CompiledChoice, CompiledChoiceEcho, CompiledDialogue,
    CompiledDivertTarget, CompiledLine, CompiledMetadataEntry, CompiledSourceMapEntry,
    CompiledStatement, CompiledStatementKind, CompiledValueError, LineIndex, MetadataIndex,
    MetadataRange, SourceMapIndex, SpeakerIndex, StatementIndex, StatementRange, TableRange,
};

use crate::error::UnsupportedStatementKind;
use crate::event::{ChoiceEchoMode, DialogueChoice, DialogueEvent, DialogueLine};
use crate::session::PendingPrompt;
use crate::{DialogueError, DialogueSession};

const MAX_INTERNAL_STEPS: usize = 10_000;

/// Start a dialogue session at the compiled default block or an explicit block.
pub fn start_scene(
    asset: &CompiledDialogue,
    block: Option<&str>,
) -> Result<DialogueSession, DialogueError> {
    ensure_supported_header(&asset.header)?;

    let block_index = match block {
        Some(block) => lookup_block(asset, block)?,
        None => asset.default_block,
    };
    let compiled_block = block_at(asset, block_index)?;

    Ok(DialogueSession::new(
        asset.header.asset_id.clone(),
        asset.header.format_version,
        asset.header.compiler_compatibility_version,
        block_index,
        compiled_block.statements.start,
    ))
}

/// Advance a session until the next public runtime event.
pub fn next(
    asset: &CompiledDialogue,
    session: &mut DialogueSession,
) -> Result<DialogueEvent, DialogueError> {
    ensure_supported_header(&asset.header)?;
    ensure_session_matches_asset(asset, session)?;

    if session.ended {
        return Err(DialogueError::SessionEnded);
    }
    if let Some(prompt) = &session.pending_prompt {
        return Err(DialogueError::PromptPending {
            choices: prompt.choice_ids.clone(),
        });
    }

    for _ in 0..MAX_INTERNAL_STEPS {
        let block = block_at(asset, session.current_block)?;
        let block_statements = statement_range(asset, block.statements)?;
        let next_statement = session.next_statement.as_u32() as usize;

        if next_statement == block_statements.end {
            session.ended = true;
            return session.emit(DialogueEvent::End);
        }
        if !block_statements.contains(&next_statement) {
            return Err(malformed(format!(
                "session statement pointer {} is outside block `{}` range {}..{}",
                session.next_statement.as_u32(),
                block.id,
                block_statements.start,
                block_statements.end
            )));
        }

        let statement = statement_at(asset, session.next_statement)?;
        match &statement.kind {
            CompiledStatementKind::Line(line) => {
                let event = DialogueEvent::Line(dialogue_line(asset, *line)?);
                session.next_statement = next_statement_after(session.next_statement)?;
                return session.emit(event);
            }
            CompiledStatementKind::Prompt { line, choices } => {
                let line = line.map(|line| dialogue_line(asset, line)).transpose()?;
                let choices = dialogue_choices(asset, *choices)?;
                let choice_ids = choices.iter().map(|choice| choice.id.clone()).collect();
                session.next_statement = next_statement_after(session.next_statement)?;
                session.previous_prompt_choices = choices
                    .iter()
                    .map(|choice| choice.id.clone())
                    .collect::<Vec<_>>();
                session.pending_prompt = Some(PendingPrompt { choice_ids });

                return session.emit(DialogueEvent::Prompt { line, choices });
            }
            CompiledStatementKind::Divert(target) => {
                if matches!(target, CompiledDivertTarget::End) {
                    session.ended = true;
                    return session.emit(DialogueEvent::End);
                }
                apply_divert(asset, session, target)?;
            }
            CompiledStatementKind::End => {
                session.ended = true;
                return session.emit(DialogueEvent::End);
            }
            CompiledStatementKind::If { .. } => {
                return Err(DialogueError::UnsupportedStatement {
                    kind: UnsupportedStatementKind::If,
                });
            }
            CompiledStatementKind::Match { .. } => {
                return Err(DialogueError::UnsupportedStatement {
                    kind: UnsupportedStatementKind::Match,
                });
            }
            CompiledStatementKind::Effect(_) => {
                return Err(DialogueError::UnsupportedStatement {
                    kind: UnsupportedStatementKind::Effect,
                });
            }
        }
    }

    Err(DialogueError::TraversalLimitExceeded {
        limit: MAX_INTERNAL_STEPS,
    })
}

fn ensure_supported_header(header: &CompiledAssetHeader) -> Result<(), DialogueError> {
    if header.format_version != COMPILED_ASSET_FORMAT_VERSION_V0
        || header.compiler_compatibility_version != COMPILER_COMPATIBILITY_VERSION_V0
    {
        return Err(DialogueError::UnsupportedCompiledFormat {
            format_version: header.format_version,
            compiler_compatibility_version: header.compiler_compatibility_version,
        });
    }

    Ok(())
}

fn ensure_session_matches_asset(
    asset: &CompiledDialogue,
    session: &DialogueSession,
) -> Result<(), DialogueError> {
    if session.asset_id != asset.header.asset_id
        || session.format_version != asset.header.format_version
        || session.compiler_compatibility_version != asset.header.compiler_compatibility_version
    {
        return Err(DialogueError::AssetMismatch {
            expected_asset_id: session.asset_id.as_str().to_owned(),
            actual_asset_id: asset.header.asset_id.as_str().to_owned(),
            expected_format_version: session.format_version,
            actual_format_version: asset.header.format_version,
            expected_compiler_compatibility_version: session.compiler_compatibility_version,
            actual_compiler_compatibility_version: asset.header.compiler_compatibility_version,
        });
    }

    Ok(())
}

fn lookup_block(asset: &CompiledDialogue, block: &str) -> Result<BlockIndex, DialogueError> {
    asset
        .block_lookup
        .as_slice()
        .binary_search_by(|entry| entry.id.as_str().cmp(block))
        .map(|index| asset.block_lookup.as_slice()[index].index)
        .map_err(|_| DialogueError::UnknownBlock {
            block: block.to_owned(),
        })
}

fn apply_divert(
    asset: &CompiledDialogue,
    session: &mut DialogueSession,
    target: &CompiledDivertTarget,
) -> Result<(), DialogueError> {
    match target {
        CompiledDivertTarget::Block(block_index) => {
            let block = block_at(asset, *block_index)?;
            session.current_block = *block_index;
            session.next_statement = block.statements.start;
        }
        CompiledDivertTarget::End => unreachable!("end diverts are handled by caller"),
    }

    Ok(())
}

fn dialogue_line(
    asset: &CompiledDialogue,
    line_index: LineIndex,
) -> Result<DialogueLine, DialogueError> {
    let line = line_at(asset, line_index)?;
    Ok(DialogueLine {
        id: line.id.clone(),
        source_text: line.source_text.clone(),
        text: line.source_text.clone(),
        speaker: line
            .speaker
            .map(|speaker| speaker_at(asset, speaker).map(|speaker| speaker.id.clone()))
            .transpose()?,
        metadata: metadata(asset, line.metadata)?,
    })
}

fn dialogue_choices(
    asset: &CompiledDialogue,
    range: ChoiceRange,
) -> Result<Vec<DialogueChoice>, DialogueError> {
    choice_range(asset, range)?
        .map(|choice| {
            if choice.condition.is_some() {
                return Err(DialogueError::UnsupportedStatement {
                    kind: UnsupportedStatementKind::ChoiceCondition,
                });
            }

            Ok(DialogueChoice {
                id: choice.id.clone(),
                source_text: choice.source_text.clone(),
                text: choice.source_text.clone(),
                metadata: metadata(asset, choice.metadata)?,
                is_available: true,
                unavailable_reason: None,
                echo: choice_echo(&choice.echo),
            })
        })
        .collect()
}

fn choice_echo(echo: &CompiledChoiceEcho) -> ChoiceEchoMode {
    match echo {
        CompiledChoiceEcho::None => ChoiceEchoMode::None,
        CompiledChoiceEcho::SelectedText => ChoiceEchoMode::SelectedText,
        CompiledChoiceEcho::ExplicitLine(line_id) => ChoiceEchoMode::ExplicitLine(line_id.clone()),
    }
}

fn metadata(
    asset: &CompiledDialogue,
    range: MetadataRange,
) -> Result<Vec<recite_core::MetadataEntry>, DialogueError> {
    metadata_range(asset, range)?
        .map(|entry| metadata_entry(asset, entry))
        .collect()
}

fn metadata_entry(
    asset: &CompiledDialogue,
    entry: &CompiledMetadataEntry,
) -> Result<recite_core::MetadataEntry, DialogueError> {
    Ok(recite_core::MetadataEntry {
        key: entry.key.clone(),
        value: entry.value.clone(),
        source_span: entry
            .source_map
            .map(|source_map| source_map_at(asset, source_map).map(|entry| entry.span.clone()))
            .transpose()?,
        key_span: None,
        value_span: None,
    })
}

fn block_at(
    asset: &CompiledDialogue,
    index: BlockIndex,
) -> Result<&recite_core::CompiledBlock, DialogueError> {
    asset
        .blocks
        .get(index.as_u32() as usize)
        .ok_or_else(|| malformed(format!("block index {} is out of range", index.as_u32())))
}

fn statement_at(
    asset: &CompiledDialogue,
    index: StatementIndex,
) -> Result<&CompiledStatement, DialogueError> {
    asset
        .statements
        .get(index.as_u32() as usize)
        .ok_or_else(|| {
            malformed(format!(
                "statement index {} is out of range",
                index.as_u32()
            ))
        })
}

fn line_at(asset: &CompiledDialogue, index: LineIndex) -> Result<&CompiledLine, DialogueError> {
    asset
        .lines
        .get(index.as_u32() as usize)
        .ok_or_else(|| malformed(format!("line index {} is out of range", index.as_u32())))
}

fn speaker_at(
    asset: &CompiledDialogue,
    index: SpeakerIndex,
) -> Result<&recite_core::CompiledSpeaker, DialogueError> {
    asset
        .speakers
        .get(index.as_u32() as usize)
        .ok_or_else(|| malformed(format!("speaker index {} is out of range", index.as_u32())))
}

fn source_map_at(
    asset: &CompiledDialogue,
    index: SourceMapIndex,
) -> Result<&CompiledSourceMapEntry, DialogueError> {
    asset
        .source_maps
        .get(index.as_u32() as usize)
        .ok_or_else(|| {
            malformed(format!(
                "source map index {} is out of range",
                index.as_u32()
            ))
        })
}

fn choice_range(
    asset: &CompiledDialogue,
    range: ChoiceRange,
) -> Result<impl Iterator<Item = &CompiledChoice>, DialogueError> {
    let bounds = table_range("choices", asset.choices.len(), range, ChoiceIndex::as_u32)?;
    Ok(asset.choices[bounds].iter())
}

fn metadata_range(
    asset: &CompiledDialogue,
    range: MetadataRange,
) -> Result<impl Iterator<Item = &CompiledMetadataEntry>, DialogueError> {
    let bounds = table_range(
        "metadata",
        asset.metadata.len(),
        range,
        MetadataIndex::as_u32,
    )?;
    Ok(asset.metadata[bounds].iter())
}

fn statement_range(
    asset: &CompiledDialogue,
    range: StatementRange,
) -> Result<Range<usize>, DialogueError> {
    table_range(
        "statements",
        asset.statements.len(),
        range,
        StatementIndex::as_u32,
    )
}

fn table_range<I: Copy>(
    table: &'static str,
    table_len: usize,
    range: TableRange<I>,
    index: impl Fn(I) -> u32,
) -> Result<Range<usize>, DialogueError> {
    let start = index(range.start) as usize;
    let len = range.len as usize;
    let end = start
        .checked_add(len)
        .ok_or_else(|| malformed(format!("{table} range overflows usize")))?;

    if end > table_len {
        return Err(malformed(format!(
            "{table} range {start}..{end} exceeds table length {table_len}"
        )));
    }

    Ok(start..end)
}

fn next_statement_after(index: StatementIndex) -> Result<StatementIndex, DialogueError> {
    index
        .as_u32()
        .checked_add(1)
        .map(StatementIndex::new)
        .ok_or_else(|| malformed("statement index overflowed".to_owned()))
}

fn malformed(reason: String) -> DialogueError {
    DialogueError::MalformedCompiledAsset { reason }
}

impl From<CompiledValueError> for DialogueError {
    fn from(error: CompiledValueError) -> Self {
        malformed(error.to_string())
    }
}
