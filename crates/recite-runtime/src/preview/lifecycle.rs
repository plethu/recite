use recite_core::{BlockId, CompiledDialogue, EffectId};

use super::PreviewSession;
use super::model::{
    PreviewConditionRequestId, PreviewEvent, PreviewOptions, PreviewOutput, PreviewState,
};
use crate::{
    DialogueError, DialogueSessionOptions, EffectAck, PreviewError, PreviewPromptIdentity,
    PreviewStatus, acknowledge_effect, start_scene_with_options,
};

impl<'asset> PreviewSession<'asset> {
    /// Starts a preview at the compiled default block or an explicit block.
    pub fn new(
        asset: &'asset CompiledDialogue,
        block: Option<&str>,
        options: PreviewOptions,
    ) -> Result<Self, DialogueError> {
        let session = start_scene_with_options(asset, block, session_options(&options))?;
        let state = PreviewState::new(asset, &session, PreviewStatus::Ready);
        let trace =
            super::model::PreviewTrace::new(session.locale().cloned(), options.variant.clone());
        Ok(Self {
            asset,
            block: block.map(str::to_owned),
            options,
            session,
            state,
            trace,
            transcript: super::model::PreviewTranscript::default(),
            pending: None,
            next_condition_id: PreviewConditionRequestId::new(0),
        })
    }

    /// Reports that a different compiled payload requires an explicit restart.
    /// The active asset is never silently replaced.
    pub fn assess_asset(&mut self, candidate: &CompiledDialogue) -> PreviewOutput {
        if self.asset == candidate {
            return PreviewOutput::new(Vec::new(), self.state.clone());
        }
        self.append_events(vec![PreviewEvent::RestartRequired {
            active_asset: self.asset.header.asset_id.clone(),
            replacement_asset: candidate.header.asset_id.clone(),
        }])
    }

    pub(super) fn acknowledge_pending(
        &mut self,
        effect_id: EffectId,
        ack: EffectAck,
    ) -> PreviewOutput {
        let mut trial = self.session.clone();
        match acknowledge_effect(&mut trial, effect_id.clone(), ack.clone()) {
            Ok(()) => {
                self.session = trial;
                self.state.status = PreviewStatus::Ready;
                self.append_events(vec![PreviewEvent::EffectAcknowledged { effect_id, ack }])
            }
            Err(error) => self.error(PreviewError::Runtime(error)),
        }
    }

    pub(super) fn restart(&mut self) -> PreviewOutput {
        match start_scene_with_options(
            self.asset,
            self.block.as_deref(),
            session_options(&self.options),
        ) {
            Ok(session) => {
                self.session = session;
                self.pending = None;
                self.state = PreviewState::new(self.asset, &self.session, PreviewStatus::Ready);
                self.append_events(vec![PreviewEvent::Restarted {
                    block: self.current_block_id_opt(),
                    locale: self.session.locale().cloned(),
                }])
            }
            Err(error) => self.error(PreviewError::Runtime(error)),
        }
    }

    pub(super) fn pending_prompt_identity(&self) -> Option<PreviewPromptIdentity> {
        match &self.state.status {
            PreviewStatus::WaitingForChoice { prompt } => Some(prompt.identity().clone()),
            _ => None,
        }
    }

    pub(super) fn current_block_id_opt(&self) -> Option<BlockId> {
        self.asset
            .blocks
            .get(self.session.current_block.as_u32() as usize)
            .map(|block| block.id.clone())
    }

    pub(super) fn prompt_identity_for_session(
        &self,
        session: &crate::DialogueSession,
    ) -> Option<PreviewPromptIdentity> {
        let block = self
            .asset
            .blocks
            .get(session.active_block_index().as_u32() as usize)?;
        let statement = self
            .asset
            .statements
            .get(session.next_statement_index().as_u32() as usize)?;
        let recite_core::CompiledStatementKind::Prompt { line, choices } = &statement.kind else {
            return None;
        };
        let start = choices.start.as_u32() as usize;
        let end = start.checked_add(choices.len as usize)?;
        let choices = self.asset.choices.get(start..end)?;
        Some(PreviewPromptIdentity {
            block: block.id.clone(),
            line: line.and_then(|index| {
                self.asset
                    .lines
                    .get(index.as_u32() as usize)
                    .map(|line| line.id.clone())
            }),
            choices: choices.iter().map(|choice| choice.id.clone()).collect(),
        })
    }
}

fn session_options(options: &PreviewOptions) -> DialogueSessionOptions {
    match options.locale.clone() {
        Some(locale) => DialogueSessionOptions::new().with_locale(locale),
        None => DialogueSessionOptions::new(),
    }
}
