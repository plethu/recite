use recite_core::{BlockId, ChoiceId, CompiledAssetId, LocaleId};

use crate::{DialogueEffectRequest, DialogueSession, DialogueSessionSnapshot};

use super::api::PreviewConditionRequest;
use super::events::PreviewPrompt;

pub const PREVIEW_SNAPSHOT_FORMAT_VERSION: u16 = 1;

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum PreviewStatus {
    Ready,
    WaitingForCondition {
        request: PreviewConditionRequest,
    },
    WaitingForChoice {
        prompt: Box<PreviewPrompt>,
    },
    WaitingForEffect {
        effect: DialogueEffectRequest,
    },
    Ended,
    RestartRequired {
        active_asset: CompiledAssetId,
        replacement_asset: CompiledAssetId,
    },
}

impl PreviewStatus {
    pub(crate) fn is_waiting_for_condition(&self) -> bool {
        matches!(self, Self::WaitingForCondition { .. })
    }
}

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub struct PreviewState {
    pub(crate) asset_id: CompiledAssetId,
    pub(crate) block: Option<BlockId>,
    pub(crate) locale: Option<LocaleId>,
    pub(crate) status: PreviewStatus,
    pub(crate) selected_choice_history: Vec<ChoiceId>,
    pub(crate) deferred_effects: Vec<DialogueEffectRequest>,
}

pub type PreviewSessionState = PreviewState;

impl PreviewState {
    pub(crate) fn new(
        asset: &recite_core::CompiledDialogue,
        session: &DialogueSession,
        status: PreviewStatus,
    ) -> Self {
        let block = asset
            .blocks
            .get(session.current_block.as_u32() as usize)
            .map(|block| block.id.clone());
        Self {
            asset_id: asset.header.asset_id.clone(),
            block,
            locale: session.locale().cloned(),
            status,
            selected_choice_history: session.selected_choice_history().to_vec(),
            deferred_effects: session.deferred_effects().to_vec(),
        }
    }

    #[must_use]
    pub fn asset_id(&self) -> &CompiledAssetId {
        &self.asset_id
    }

    #[must_use]
    pub fn block(&self) -> Option<&BlockId> {
        self.block.as_ref()
    }

    #[must_use]
    pub fn locale(&self) -> Option<&LocaleId> {
        self.locale.as_ref()
    }

    #[must_use]
    pub fn status(&self) -> &PreviewStatus {
        &self.status
    }

    #[must_use]
    pub fn selected_choice_history(&self) -> &[ChoiceId] {
        &self.selected_choice_history
    }

    #[must_use]
    pub fn deferred_effects(&self) -> &[DialogueEffectRequest] {
        &self.deferred_effects
    }
}

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub struct PreviewSnapshot {
    pub(crate) snapshot_format_version: u16,
    pub(crate) session: DialogueSessionSnapshot,
    /// The last control/prompt projection is snapshotted while trace and
    /// transcript remain session-local diagnostic history.
    pub(crate) state: PreviewState,
}

impl PreviewSnapshot {
    #[must_use]
    pub fn snapshot_format_version(&self) -> u16 {
        self.snapshot_format_version
    }

    #[must_use]
    pub fn session(&self) -> &DialogueSessionSnapshot {
        &self.session
    }

    #[must_use]
    pub fn state(&self) -> &PreviewState {
        &self.state
    }
}
