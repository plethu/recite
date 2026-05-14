mod asset;
mod output;

use recite_core::{
    ChoiceId, ChoiceRange, CompiledDialogue, CompiledDivertTarget, CompiledStatementKind,
    StatementIndex,
};

use crate::error::UnsupportedStatementKind;
use crate::event::DialogueEvent;
use crate::session::{PendingPrompt, PendingPromptChoice};
use crate::{DialogueError, DialogueSession};

use self::asset::{AssetView, malformed};
use self::output::{dialogue_choices, dialogue_line};

const MAX_INTERNAL_STEPS: usize = 10_000;

/// Start a dialogue session at the compiled default block or an explicit block.
pub fn start_scene(
    asset: &CompiledDialogue,
    block: Option<&str>,
) -> Result<DialogueSession, DialogueError> {
    let asset_view = AssetView::new(asset)?;

    let block_index = match block {
        Some(block) => asset_view.lookup_block(block)?,
        None => asset_view.default_block(),
    };
    let compiled_block = asset_view.block_at(block_index)?;

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
    let asset_view = AssetView::new(asset)?;
    asset_view.ensure_session_matches(session)?;

    if session.ended {
        return Err(DialogueError::SessionEnded);
    }
    if let Some(prompt) = &session.pending_prompt {
        return Err(DialogueError::PromptPending {
            choices: prompt.choice_ids(),
        });
    }

    for _ in 0..MAX_INTERNAL_STEPS {
        let block = asset_view.block_at(session.current_block)?;
        let block_statements = asset_view.statement_range(block.statements)?;
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

        let statement = asset_view.statement_at(session.next_statement)?;
        match &statement.kind {
            CompiledStatementKind::Line(line) => {
                let event =
                    DialogueEvent::Line(dialogue_line(asset_view, *line, block.default_speaker)?);
                session.next_statement = next_statement_after(session.next_statement)?;
                return session.emit(event);
            }
            CompiledStatementKind::Prompt { line, choices } => {
                let choice_range = *choices;
                if choice_range.is_empty() {
                    return Err(malformed(format!(
                        "prompt statement at index {} has no choices",
                        session.next_statement.as_u32()
                    )));
                }

                let line = line
                    .map(|line| dialogue_line(asset_view, line, block.default_speaker))
                    .transpose()?;
                let choices = dialogue_choices(asset_view, choice_range)?;
                let pending_choices = pending_prompt_choices(asset_view, choice_range)?;
                let choice_ids = choices.iter().map(|choice| choice.id.clone()).collect();
                session.next_statement = next_statement_after(session.next_statement)?;
                session.previous_prompt_choices = choice_ids;
                session.pending_prompt = Some(PendingPrompt {
                    choices: pending_choices,
                });

                return session.emit(DialogueEvent::Prompt { line, choices });
            }
            CompiledStatementKind::Divert(target) => {
                if matches!(target, CompiledDivertTarget::End) {
                    session.ended = true;
                    return session.emit(DialogueEvent::End);
                }

                apply_divert(asset_view, session, target)?;
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

/// Select a pending prompt choice by stable choice ID and continue traversal.
pub fn choose(
    asset: &CompiledDialogue,
    session: &mut DialogueSession,
    choice_id: ChoiceId,
) -> Result<DialogueEvent, DialogueError> {
    let asset_view = AssetView::new(asset)?;
    asset_view.ensure_session_matches(session)?;

    let Some(prompt) = &session.pending_prompt else {
        return Err(DialogueError::NoPromptPending { choice: choice_id });
    };
    let Some(choice) = prompt
        .choices
        .iter()
        .find(|choice| choice.id == choice_id)
        .cloned()
    else {
        return Err(DialogueError::InvalidChoice {
            choice: choice_id,
            available_choices: prompt.choice_ids(),
        });
    };

    if !choice.is_available {
        return Err(DialogueError::UnavailableChoice {
            choice: choice.id,
            reason: choice.unavailable_reason,
        });
    }

    let next_location = match choice.target {
        CompiledDivertTarget::Block(block_index) => {
            let block = asset_view.block_at(block_index)?;
            Some((block_index, block.statements.start))
        }
        CompiledDivertTarget::End => None,
    };

    session.pending_prompt = None;
    session.selected_choice_history.push(choice.id);

    if let Some((block_index, statement_index)) = next_location {
        session.current_block = block_index;
        session.next_statement = statement_index;
        return next(asset, session);
    }

    session.ended = true;
    session.emit(DialogueEvent::End)
}

fn pending_prompt_choices(
    asset: AssetView<'_>,
    range: ChoiceRange,
) -> Result<Vec<PendingPromptChoice>, DialogueError> {
    asset
        .choices(range)?
        .iter()
        .map(|choice| {
            Ok(PendingPromptChoice {
                id: choice.id.clone(),
                target: choice.target.clone(),
                is_available: true,
                unavailable_reason: None,
            })
        })
        .collect()
}

fn apply_divert(
    asset: AssetView<'_>,
    session: &mut DialogueSession,
    target: &CompiledDivertTarget,
) -> Result<(), DialogueError> {
    match target {
        CompiledDivertTarget::Block(block_index) => {
            let block = asset.block_at(*block_index)?;
            session.current_block = *block_index;
            session.next_statement = block.statements.start;
        }
        CompiledDivertTarget::End => unreachable!("end diverts are handled by caller"),
    }

    Ok(())
}

fn next_statement_after(index: StatementIndex) -> Result<StatementIndex, DialogueError> {
    index
        .as_u32()
        .checked_add(1)
        .map(StatementIndex::new)
        .ok_or_else(|| malformed("statement index overflowed".to_owned()))
}

#[cfg(test)]
mod tests {
    use recite_core::{
        BlockIndex, BlockLookupTable, ChoiceId, ChoiceLookupTable, CompiledAssetHeader,
        CompiledAssetId, CompiledDialogue, CompiledDivertTarget, CompilerVersion, LineLookupTable,
        SchemaFingerprint, SourceMapId, StatementIndex,
    };

    use crate::session::{PendingPrompt, PendingPromptChoice};
    use crate::{DialogueError, DialogueSession};

    use super::choose;

    #[test]
    fn unavailable_pending_choice_is_structured_error_without_mutating_session() {
        let asset = empty_asset();
        let choice_id = ChoiceId::new("locked_choice").expect("valid choice ID");
        let mut session = DialogueSession::new(
            asset.header.asset_id.clone(),
            asset.header.format_version,
            asset.header.compiler_compatibility_version,
            BlockIndex::new(0),
            StatementIndex::new(0),
        );
        session.pending_prompt = Some(PendingPrompt {
            choices: vec![PendingPromptChoice {
                id: choice_id.clone(),
                target: CompiledDivertTarget::End,
                is_available: false,
                unavailable_reason: Some("missing trust".to_owned()),
            }],
        });

        assert_eq!(
            choose(&asset, &mut session, choice_id.clone()),
            Err(DialogueError::UnavailableChoice {
                choice: choice_id.clone(),
                reason: Some("missing trust".to_owned())
            })
        );
        assert_eq!(
            session
                .pending_prompt
                .as_ref()
                .map(PendingPrompt::choice_ids),
            Some(vec![choice_id])
        );
        assert!(session.selected_choice_history().is_empty());
    }

    fn empty_asset() -> CompiledDialogue {
        CompiledDialogue {
            header: CompiledAssetHeader::messagepack_v0(
                CompilerVersion::new("0.0.1").expect("valid compiler version"),
                CompiledAssetId::new("dialogue/main.recitec").expect("valid asset id"),
                SourceMapId::new("dialogue/main.recitec.map").expect("valid source map id"),
                SchemaFingerprint::NoSchema,
            ),
            default_block: BlockIndex::new(0),
            sources: Vec::new(),
            blocks: Vec::new(),
            statements: Vec::new(),
            match_arms: Vec::new(),
            lines: Vec::new(),
            choices: Vec::new(),
            speakers: Vec::new(),
            metadata: Vec::new(),
            effects: Vec::new(),
            source_maps: Vec::new(),
            block_lookup: BlockLookupTable::default(),
            line_lookup: LineLookupTable::default(),
            choice_lookup: ChoiceLookupTable::default(),
        }
    }
}
