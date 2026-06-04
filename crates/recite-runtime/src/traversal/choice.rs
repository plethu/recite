use recite_core::{ChoiceId, ChoiceRange, CompiledDialogue, CompiledDivertTarget};

use crate::context::DialogueContext;
use crate::event::{DialogueChoice, DialogueEvent};
use crate::session::PendingPromptChoice;
use crate::{DialogueError, DialogueSession};

use super::AssetView;
use super::advance::next_with_locale;
use super::availability::choice_availability;
use super::flow::finish_scene;
use super::output::{LocaleLookup, LocaleResolution, dialogue_choice};

pub fn choose(
    asset: &CompiledDialogue,
    session: &mut DialogueSession,
    choice_id: ChoiceId,
    context: &dyn DialogueContext,
) -> Result<DialogueEvent, DialogueError> {
    choose_with_locale(asset, session, choice_id, context, LocaleLookup::source())
}

/// Selects a pending choice with explicit locale resolution options.
///
/// Use [`LocaleResolution::new`] for source-text output, or attach a locale
/// provider and optional variant to resolve text against the session locale.
pub fn choose_with(
    asset: &CompiledDialogue,
    session: &mut DialogueSession,
    choice_id: ChoiceId,
    context: &dyn DialogueContext,
    locale_resolution: LocaleResolution<'_>,
) -> Result<DialogueEvent, DialogueError> {
    let locale = session.locale().cloned();

    choose_with_locale(
        asset,
        session,
        choice_id,
        context,
        LocaleLookup::from_resolution(locale.as_ref(), locale_resolution),
    )
}

fn choose_with_locale(
    asset: &CompiledDialogue,
    session: &mut DialogueSession,
    choice_id: ChoiceId,
    context: &dyn DialogueContext,
    locale: LocaleLookup<'_>,
) -> Result<DialogueEvent, DialogueError> {
    let asset_view = AssetView::new(asset)?;
    asset_view.ensure_session_matches(session)?;

    if let Some(effect) = &session.pending_effect {
        return Err(DialogueError::EffectPending {
            effect: effect.request.id.clone(),
        });
    }

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
            availability: Box::new(choice.availability),
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
        return next_with_locale(asset, session, context, locale);
    }

    finish_scene(session)
}

pub(super) struct PromptChoices {
    pub(super) events: Vec<DialogueChoice>,
    pub(super) pending: Vec<PendingPromptChoice>,
}

pub(super) fn prompt_choices(
    asset: AssetView<'_>,
    range: ChoiceRange,
    context: &dyn DialogueContext,
    locale: LocaleLookup<'_>,
) -> Result<PromptChoices, DialogueError> {
    let mut events = Vec::new();
    let mut pending = Vec::new();

    for choice in asset.choices(range)? {
        let availability = choice_availability(
            asset,
            choice.availability_requirement.as_ref(),
            choice.availability_requirement_source_text.as_deref(),
            choice.availability_reason_override.as_ref(),
            context,
            locale,
        )?;

        events.push(dialogue_choice(
            asset,
            choice,
            availability.clone(),
            locale,
        )?);
        pending.push(PendingPromptChoice {
            id: choice.id.clone(),
            target: choice.target.clone(),
            is_available: availability.is_available,
            availability,
        });
    }

    Ok(PromptChoices { events, pending })
}

#[cfg(test)]
mod tests;
