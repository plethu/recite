use recite_core::compiled::{
    CompiledDialogue, CompiledDivertTarget, CompiledMatchPattern, CompiledStatementKind,
    StatementRange,
};

use crate::context::DialogueContext;
use crate::event::DialogueEvent;
use crate::session::{PendingPrompt, SessionPhase};
use crate::{DialogueError, DialogueSession};

use super::choice::prompt_choices;
use super::condition::{evaluate_condition, evaluate_enum_condition};
use super::effect::handle_effect;
use super::flow::{apply_divert, enter_statement_range, finish_scene, next_statement_after};
use super::output::{LocaleLookup, LocaleResolution, dialogue_choice, dialogue_line};
use super::{AssetView, malformed};

const MAX_INTERNAL_STEPS: usize = 10_000;

pub fn next(
    asset: &CompiledDialogue,
    session: &mut DialogueSession,
    context: &dyn DialogueContext,
) -> Result<DialogueEvent, DialogueError> {
    next_with_locale(asset, session, context, LocaleLookup::source())
}

/// Advances traversal with explicit locale resolution options.
///
/// Use [`LocaleResolution::new`] for source-text output, or attach a locale
/// provider and optional variant to resolve text against the session locale.
pub fn next_with(
    asset: &CompiledDialogue,
    session: &mut DialogueSession,
    context: &dyn DialogueContext,
    locale_resolution: LocaleResolution<'_>,
) -> Result<DialogueEvent, DialogueError> {
    let locale = session.locale().cloned();

    next_with_locale(
        asset,
        session,
        context,
        LocaleLookup::from_resolution(locale.as_ref(), locale_resolution),
    )
}

pub(super) fn next_with_locale(
    asset: &CompiledDialogue,
    session: &mut DialogueSession,
    context: &dyn DialogueContext,
    locale: LocaleLookup<'_>,
) -> Result<DialogueEvent, DialogueError> {
    let asset_view = AssetView::new(asset)?;
    asset_view.ensure_session_matches(session)?;

    match &mut session.phase {
        SessionPhase::Running => {}
        SessionPhase::Ended => return Err(DialogueError::SessionEnded),
        SessionPhase::AwaitingEffect(effect) => {
            if effect.reemit_on_next {
                effect.reemit_on_next = false;
                return Ok(DialogueEvent::Effect(effect.request.clone()));
            }
            return Err(DialogueError::EffectPending {
                effect: effect.request.id.clone(),
            });
        }
        SessionPhase::AwaitingChoice(prompt) => {
            if prompt.reemit_on_next {
                prompt.reemit_on_next = false;
                let prompt = prompt.clone();
                return represent_prompt(asset_view, session, &prompt, locale);
            }
            return Err(DialogueError::PromptPending {
                choices: prompt.choice_ids(),
            });
        }
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
                let event = DialogueEvent::Line(dialogue_line(
                    asset_view,
                    *line,
                    block.default_speaker,
                    locale,
                )?);
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
                    .map(|line| dialogue_line(asset_view, line, block.default_speaker, locale))
                    .transpose()?;
                let prompt_choices = prompt_choices(asset_view, choice_range, context, locale)?;
                let choice_ids = prompt_choices
                    .events
                    .iter()
                    .map(|choice| choice.id.clone())
                    .collect();
                let prompt_statement = session.next_statement;
                session.next_statement = next_statement_after(session.next_statement)?;
                session.previous_prompt_choices = choice_ids;
                session.phase = SessionPhase::AwaitingChoice(PendingPrompt {
                    statement: prompt_statement,
                    choices: prompt_choices.pending,
                    reemit_on_next: false,
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
            CompiledStatementKind::Match { scrutinee, arms } => {
                let variant = evaluate_enum_condition(context, scrutinee)?;
                let selected_range = select_match_arm(asset_view, *arms, &variant)?;
                let continuation = next_statement_after(session.next_statement)?;
                enter_statement_range(asset_view, session, selected_range, continuation)?;
            }
            CompiledStatementKind::Effect(effect_index) => {
                if let Some(event) = handle_effect(asset_view, session, *effect_index)? {
                    return session.emit(event);
                }
            }
        }
    }

    Err(DialogueError::TraversalLimitExceeded {
        limit: MAX_INTERNAL_STEPS,
    })
}

/// Presents a restored prompt again with its saved availability. Like a
/// restored blocking effect, the repeat does not advance the trace counter,
/// and conditions are not queried again.
fn represent_prompt(
    asset: AssetView<'_>,
    session: &DialogueSession,
    prompt: &PendingPrompt,
    locale: LocaleLookup<'_>,
) -> Result<DialogueEvent, DialogueError> {
    let block = asset.block_at(session.current_block)?;
    let CompiledStatementKind::Prompt { line, choices } =
        &asset.statement_at(prompt.statement)?.kind
    else {
        return Err(malformed(format!(
            "pending prompt statement {} is not a prompt",
            prompt.statement.as_u32()
        )));
    };
    let line = line
        .map(|line| dialogue_line(asset, line, block.default_speaker, locale))
        .transpose()?;
    let compiled = asset.choices(*choices)?;
    let mut events = Vec::with_capacity(prompt.choices.len());
    for pending in &prompt.choices {
        let Some(choice) = compiled.iter().find(|choice| choice.id == pending.id) else {
            return Err(malformed(format!(
                "pending prompt choice `{}` is not in its compiled prompt",
                pending.id
            )));
        };
        events.push(dialogue_choice(
            asset,
            choice,
            pending.availability.clone(),
            locale,
        )?);
    }
    Ok(DialogueEvent::Prompt {
        line,
        choices: events,
    })
}

fn select_match_arm(
    asset: AssetView<'_>,
    arms: recite_core::compiled::MatchArmRange,
    variant: &str,
) -> Result<StatementRange, DialogueError> {
    let mut wildcard_range = None;

    for arm in asset.match_arms(arms)? {
        match &arm.pattern {
            CompiledMatchPattern::Variant(value) if value == variant => {
                return Ok(arm.statements);
            }
            CompiledMatchPattern::Wildcard if wildcard_range.is_none() => {
                wildcard_range = Some(arm.statements);
            }
            CompiledMatchPattern::Variant(_) | CompiledMatchPattern::Wildcard => {}
        }
    }

    wildcard_range.ok_or_else(|| {
        malformed(format!(
            "match statement has no arm for enum variant `{variant}` and no wildcard arm"
        ))
    })
}
