mod asset;
mod output;

use recite_core::{
    ChoiceId, ChoiceRange, CompiledConditionCall, CompiledConditionExpression, CompiledDialogue,
    CompiledDivertTarget, CompiledEffectMode, CompiledStatementKind, StatementIndex,
    StatementRange,
};

use crate::context::{ConditionQuery, DialogueContext};
use crate::error::UnsupportedStatementKind;
use crate::event::{DialogueChoice, DialogueEvent};
use crate::session::{PendingPrompt, PendingPromptChoice, StatementFrame};
use crate::{DialogueError, DialogueSession};

use self::asset::{AssetView, malformed};
use self::output::{dialogue_choice, dialogue_effect_request, dialogue_line, effect_mode};

const MAX_INTERNAL_STEPS: usize = 10_000;
const MAX_CONDITION_DEPTH: usize = 128;

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
        compiled_block.statements,
    ))
}

/// Advance a session until the next public runtime event.
pub fn next(
    asset: &CompiledDialogue,
    session: &mut DialogueSession,
    context: &dyn DialogueContext,
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
        let current_statements = asset_view.statement_range(session.current_range)?;
        let next_statement = session.next_statement.as_u32() as usize;

        if next_statement == current_statements.end {
            if let Some(frame) = session.continuation_stack.pop() {
                session.current_range = frame.range;
                session.next_statement = frame.next_statement;
                continue;
            }

            return finish_scene(session);
        }
        if !current_statements.contains(&next_statement) {
            return Err(malformed(format!(
                "session statement pointer {} is outside active range {}..{} in block `{}`",
                session.next_statement.as_u32(),
                current_statements.start,
                current_statements.end,
                block.id,
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
                let prompt_choices = prompt_choices(asset_view, choice_range, context)?;
                let choice_ids = prompt_choices
                    .events
                    .iter()
                    .map(|choice| choice.id.clone())
                    .collect();
                session.next_statement = next_statement_after(session.next_statement)?;
                session.previous_prompt_choices = choice_ids;
                session.pending_prompt = Some(PendingPrompt {
                    choices: prompt_choices.pending,
                });

                return session.emit(DialogueEvent::Prompt {
                    line,
                    choices: prompt_choices.events,
                });
            }
            CompiledStatementKind::Divert(target) => {
                if matches!(target, CompiledDivertTarget::End) {
                    return finish_scene(session);
                }

                apply_divert(asset_view, session, target)?;
            }
            CompiledStatementKind::End => {
                return finish_scene(session);
            }
            CompiledStatementKind::If {
                condition,
                then_statements,
                else_statements,
            } => {
                let condition_is_true = evaluate_condition(context, condition)?;
                let selected_range = if condition_is_true {
                    *then_statements
                } else {
                    *else_statements
                };
                let continuation = next_statement_after(session.next_statement)?;
                enter_statement_range(asset_view, session, selected_range, continuation)?;
            }
            CompiledStatementKind::Match { .. } => {
                return Err(DialogueError::UnsupportedStatement {
                    kind: UnsupportedStatementKind::Match,
                });
            }
            CompiledStatementKind::Effect(effect_index) => {
                let effect = asset_view.effect_at(*effect_index)?;
                let mode = effect_mode(effect.mode);
                if !matches!(effect.mode, CompiledEffectMode::Deferred) {
                    return Err(DialogueError::UnsupportedEffectMode { mode });
                }

                let next_statement = next_statement_after(session.next_statement)?;
                session
                    .deferred_effects
                    .push(dialogue_effect_request(asset_view, effect)?);
                session.next_statement = next_statement;
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
    context: &dyn DialogueContext,
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
            prompt_choices: prompt.choice_ids(),
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
        session.current_range = asset_view.block_at(block_index)?.statements;
        session.next_statement = statement_index;
        session.continuation_stack.clear();
        return next(asset, session, context);
    }

    finish_scene(session)
}

fn finish_scene(session: &mut DialogueSession) -> Result<DialogueEvent, DialogueError> {
    session.ended = true;
    session.continuation_stack.clear();
    let deferred_effects = session.deferred_effects.clone();
    session.emit(DialogueEvent::End { deferred_effects })
}

struct PromptChoices {
    events: Vec<DialogueChoice>,
    pending: Vec<PendingPromptChoice>,
}

fn prompt_choices(
    asset: AssetView<'_>,
    range: ChoiceRange,
    context: &dyn DialogueContext,
) -> Result<PromptChoices, DialogueError> {
    let mut events = Vec::new();
    let mut pending = Vec::new();

    for choice in asset.choices(range)? {
        let is_available = match &choice.condition {
            Some(condition) => evaluate_condition(context, condition)?,
            None => true,
        };
        let unavailable_reason = None;

        events.push(dialogue_choice(
            asset,
            choice,
            is_available,
            unavailable_reason.clone(),
        )?);
        pending.push(PendingPromptChoice {
            id: choice.id.clone(),
            target: choice.target.clone(),
            is_available,
            unavailable_reason,
        });
    }

    Ok(PromptChoices { events, pending })
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
            session.current_range = block.statements;
            session.next_statement = block.statements.start;
            session.continuation_stack.clear();
        }
        CompiledDivertTarget::End => unreachable!("end diverts are handled by caller"),
    }

    Ok(())
}

fn enter_statement_range(
    asset: AssetView<'_>,
    session: &mut DialogueSession,
    range: StatementRange,
    continuation: StatementIndex,
) -> Result<(), DialogueError> {
    asset.statement_range(range)?;
    session.continuation_stack.push(StatementFrame {
        range: session.current_range,
        next_statement: continuation,
    });
    session.current_range = range;
    session.next_statement = range.start;

    Ok(())
}

fn evaluate_condition(
    context: &dyn DialogueContext,
    condition: &CompiledConditionExpression,
) -> Result<bool, DialogueError> {
    evaluate_condition_at_depth(context, condition, 0)
}

fn evaluate_condition_at_depth(
    context: &dyn DialogueContext,
    condition: &CompiledConditionExpression,
    depth: usize,
) -> Result<bool, DialogueError> {
    if depth > MAX_CONDITION_DEPTH {
        return Err(DialogueError::ConditionDepthLimitExceeded {
            limit: MAX_CONDITION_DEPTH,
        });
    }

    match condition {
        CompiledConditionExpression::Call(call) => evaluate_condition_call(context, call),
        CompiledConditionExpression::And(expressions) => {
            if expressions.is_empty() {
                return Err(malformed(
                    "condition `and` expression has no children".to_owned(),
                ));
            }

            for expression in expressions {
                if !evaluate_condition_at_depth(context, expression, depth + 1)? {
                    return Ok(false);
                }
            }

            Ok(true)
        }
        CompiledConditionExpression::Or(expressions) => {
            if expressions.is_empty() {
                return Err(malformed(
                    "condition `or` expression has no children".to_owned(),
                ));
            }

            for expression in expressions {
                if evaluate_condition_at_depth(context, expression, depth + 1)? {
                    return Ok(true);
                }
            }

            Ok(false)
        }
        CompiledConditionExpression::Not(expression) => Ok(!evaluate_condition_at_depth(
            context,
            expression,
            depth + 1,
        )?),
    }
}

fn evaluate_condition_call(
    context: &dyn DialogueContext,
    call: &CompiledConditionCall,
) -> Result<bool, DialogueError> {
    context
        .evaluate_condition(ConditionQuery::new(&call.function, &call.args))
        .map_err(|error| DialogueError::ConditionEvaluationFailed {
            function: call.function.clone(),
            reason: error.reason().to_owned(),
        })
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
        SchemaFingerprint, SourceMapId, StatementIndex, StatementRange,
    };

    use crate::session::{PendingPrompt, PendingPromptChoice};
    use crate::{DialogueError, DialogueSession, EmptyDialogueContext};

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
            StatementRange::new(StatementIndex::new(0), 0),
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
            choose(
                &asset,
                &mut session,
                choice_id.clone(),
                &EmptyDialogueContext
            ),
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
