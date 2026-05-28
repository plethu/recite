use recite_core::{
    CompiledDialogue, CompiledDivertTarget, CompiledMatchPattern, CompiledStatementKind,
    StatementRange,
};

use crate::context::DialogueContext;
use crate::event::DialogueEvent;
use crate::locale::LocaleProvider;
use crate::session::PendingPrompt;
use crate::{DialogueError, DialogueSession};

use super::choice::prompt_choices;
use super::condition::{evaluate_condition, evaluate_enum_condition};
use super::effect::handle_effect;
use super::flow::{apply_divert, enter_statement_range, finish_scene, next_statement_after};
use super::output::{LocaleLookup, dialogue_line};
use super::{AssetView, malformed};

const MAX_INTERNAL_STEPS: usize = 10_000;

pub fn next(
    asset: &CompiledDialogue,
    session: &mut DialogueSession,
    context: &dyn DialogueContext,
) -> Result<DialogueEvent, DialogueError> {
    next_with_locale(asset, session, context, LocaleLookup::source())
}

pub fn next_with_locale_provider(
    asset: &CompiledDialogue,
    session: &mut DialogueSession,
    context: &dyn DialogueContext,
    provider: &dyn LocaleProvider,
) -> Result<DialogueEvent, DialogueError> {
    next_with_locale_provider_and_variant(asset, session, context, provider, None)
}

pub fn next_with_locale_provider_and_variant(
    asset: &CompiledDialogue,
    session: &mut DialogueSession,
    context: &dyn DialogueContext,
    provider: &dyn LocaleProvider,
    variant: Option<&str>,
) -> Result<DialogueEvent, DialogueError> {
    let locale = session.locale().cloned();
    let variant = variant.map(str::to_owned);

    next_with_locale(
        asset,
        session,
        context,
        LocaleLookup {
            locale: locale.as_ref(),
            variant: variant.as_deref(),
            provider: Some(provider),
        },
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

    if session.ended {
        return Err(DialogueError::SessionEnded);
    }
    if let Some(effect) = &mut session.pending_effect {
        if effect.reemit_on_next {
            effect.reemit_on_next = false;
            return Ok(DialogueEvent::Effect(effect.request.clone()));
        }

        return Err(DialogueError::EffectPending {
            effect: effect.request.id.clone(),
        });
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
                session.pending_prompt = Some(PendingPrompt {
                    statement: prompt_statement,
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

fn select_match_arm(
    asset: AssetView<'_>,
    arms: recite_core::MatchArmRange,
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
