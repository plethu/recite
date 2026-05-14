mod asset;
mod output;

use recite_core::{CompiledDialogue, CompiledDivertTarget, CompiledStatementKind, StatementIndex};

use crate::error::UnsupportedStatementKind;
use crate::event::DialogueEvent;
use crate::session::PendingPrompt;
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
            choices: prompt.choice_ids.clone(),
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
                if choices.is_empty() {
                    return Err(malformed(format!(
                        "prompt statement at index {} has no choices",
                        session.next_statement.as_u32()
                    )));
                }

                let line = line
                    .map(|line| dialogue_line(asset_view, line, block.default_speaker))
                    .transpose()?;
                let choices = dialogue_choices(asset_view, *choices)?;
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
