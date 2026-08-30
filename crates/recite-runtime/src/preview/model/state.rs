use recite_core::{BlockId, ChoiceId, CompiledAssetId, LocaleId};

use crate::{DialogueEffectRequest, DialogueSession, DialogueSessionSnapshot};

use super::api::PreviewConditionRequest;
use super::events::PreviewPrompt;

/// The first persisted preview snapshot format. Runtime event values are not
/// wire types and are intentionally excluded from this versioned contract.
pub const PREVIEW_SNAPSHOT_FORMAT_VERSION: u16 = 1;

#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewRestartRequirement {
    active_asset: CompiledAssetId,
    replacement_asset: CompiledAssetId,
}

impl PreviewRestartRequirement {
    pub(crate) fn new(active_asset: CompiledAssetId, replacement_asset: CompiledAssetId) -> Self {
        Self {
            active_asset,
            replacement_asset,
        }
    }

    #[must_use]
    pub fn active_asset(&self) -> &CompiledAssetId {
        &self.active_asset
    }

    #[must_use]
    pub fn replacement_asset(&self) -> &CompiledAssetId {
        &self.replacement_asset
    }
}

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum PreviewStatus {
    Ready,
    WaitingForCondition { request: PreviewConditionRequest },
    WaitingForChoice { prompt: Box<PreviewPrompt> },
    WaitingForEffect { effect: DialogueEffectRequest },
    Ended,
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
    pub(crate) restart_required: Option<PreviewRestartRequirement>,
}

pub type PreviewSessionState = PreviewState;

impl PreviewState {
    pub(crate) fn from_parts(
        asset_id: CompiledAssetId,
        block: Option<BlockId>,
        locale: Option<LocaleId>,
        status: PreviewStatus,
        selected_choice_history: Vec<ChoiceId>,
        deferred_effects: Vec<DialogueEffectRequest>,
        restart_required: Option<PreviewRestartRequirement>,
    ) -> Self {
        Self {
            asset_id,
            block,
            locale,
            status,
            selected_choice_history,
            deferred_effects,
            restart_required,
        }
    }

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
            restart_required: None,
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

    #[must_use]
    pub fn restart_required(&self) -> Option<&PreviewRestartRequirement> {
        self.restart_required.as_ref()
    }
}

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub struct PreviewSnapshot {
    pub(crate) snapshot_format_version: u16,
    pub(crate) session: DialogueSessionSnapshot,
    pub(crate) initial_block: Option<String>,
    pub(crate) options: super::api::PreviewOptions,
    pub(crate) next_condition_id: super::api::PreviewConditionRequestId,
    pub(crate) projection_fingerprint: String,
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

    #[must_use]
    pub fn initial_block(&self) -> Option<&str> {
        self.initial_block.as_deref()
    }

    #[must_use]
    pub fn options(&self) -> &super::api::PreviewOptions {
        &self.options
    }

    #[must_use]
    pub fn next_condition_request_id(&self) -> super::api::PreviewConditionRequestId {
        self.next_condition_id
    }

    #[must_use]
    pub fn projection_fingerprint(&self) -> &str {
        &self.projection_fingerprint
    }
}
