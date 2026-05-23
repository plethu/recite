use recite_core::{ChoiceId, ChoiceRange, CompiledDialogue, CompiledDivertTarget};

use crate::context::DialogueContext;
use crate::event::{DialogueChoice, DialogueEvent};
use crate::locale::LocaleProvider;
use crate::session::PendingPromptChoice;
use crate::{DialogueError, DialogueSession};

use super::AssetView;
use super::advance::next_with_locale;
use super::condition::evaluate_condition;
use super::flow::finish_scene;
use super::output::{LocaleLookup, dialogue_choice};

pub fn choose(
    asset: &CompiledDialogue,
    session: &mut DialogueSession,
    choice_id: ChoiceId,
    context: &dyn DialogueContext,
) -> Result<DialogueEvent, DialogueError> {
    choose_with_locale(asset, session, choice_id, context, LocaleLookup::source())
}

pub fn choose_with_locale_provider(
    asset: &CompiledDialogue,
    session: &mut DialogueSession,
    choice_id: ChoiceId,
    context: &dyn DialogueContext,
    provider: &dyn LocaleProvider,
) -> Result<DialogueEvent, DialogueError> {
    choose_with_locale_provider_and_variant(asset, session, choice_id, context, provider, None)
}

pub fn choose_with_locale_provider_and_variant(
    asset: &CompiledDialogue,
    session: &mut DialogueSession,
    choice_id: ChoiceId,
    context: &dyn DialogueContext,
    provider: &dyn LocaleProvider,
    variant: Option<&str>,
) -> Result<DialogueEvent, DialogueError> {
    let locale = session.locale().cloned();
    let variant = variant.map(str::to_owned);

    choose_with_locale(
        asset,
        session,
        choice_id,
        context,
        LocaleLookup {
            locale: locale.as_ref(),
            variant: variant.as_deref(),
            provider: Some(provider),
        },
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
            locale,
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

#[cfg(test)]
mod tests {
    use recite_core::{
        BlockIndex, BlockLookupTable, ChoiceId, ChoiceLookupTable, CompiledAssetHeader,
        CompiledAssetId, CompiledDialogue, CompiledDivertTarget, CompilerVersion, LineLookupTable,
        SchemaFingerprint, SourceMapId, StatementIndex, StatementRange,
    };

    use crate::session::{PendingPrompt, PendingPromptChoice};
    use crate::{DialogueError, DialogueSession, DialogueSessionOptions, EmptyDialogueContext};

    use super::choose;

    #[test]
    fn unavailable_pending_choice_is_structured_error_without_mutating_session() {
        let asset = empty_asset();
        let choice_id = ChoiceId::new("locked_choice").expect("valid choice ID");
        let mut session = DialogueSession::new(
            &asset.header,
            asset.sources.clone(),
            BlockIndex::new(0),
            StatementRange::new(StatementIndex::new(0), 0),
            DialogueSessionOptions::default(),
        );
        session.pending_prompt = Some(PendingPrompt {
            statement: StatementIndex::new(0),
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
