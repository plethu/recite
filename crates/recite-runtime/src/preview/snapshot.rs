use recite_core::LocaleId;

use super::PreviewSession;
use super::model::{
    PREVIEW_SNAPSHOT_FORMAT_VERSION, PreviewAssetRevision, PreviewError, PreviewEvent,
    PreviewSnapshot, PreviewStatus,
};
use super::snapshot_validation::state_matches_session;
use crate::{restore_session, snapshot_session};

impl<'asset> PreviewSession<'asset> {
    /// Captures stable runtime state. Trace and transcript are diagnostic
    /// projections and are deliberately excluded from the snapshot. The
    /// typed control projection is persisted because provider-derived text
    /// cannot be reconstructed without host inputs.
    pub fn snapshot(&self) -> Result<PreviewSnapshot, PreviewError> {
        if self.pending.is_some() {
            return Err(PreviewError::SnapshotPendingCondition);
        }
        let asset_revision = PreviewAssetRevision::from_asset(self.asset).map_err(|error| {
            PreviewError::AssetRevisionFailed {
                reason: error.to_string(),
            }
        })?;
        Ok(PreviewSnapshot {
            snapshot_format_version: PREVIEW_SNAPSHOT_FORMAT_VERSION,
            asset_revision,
            session: snapshot_session(&self.session),
            initial_block: self.block.clone(),
            options: self.options.clone(),
            next_condition_id: self.next_condition_id,
            state: self.state.clone(),
        })
    }

    /// Restores stable runtime state without importing trace or transcript.
    /// Snapshots are corruption-sensitive persistence, not authenticated save
    /// data; hosts must authenticate them when an attacker can edit bytes.
    pub fn restore(
        &mut self,
        snapshot: PreviewSnapshot,
    ) -> Result<super::model::PreviewOutput, PreviewError> {
        if snapshot.snapshot_format_version() != PREVIEW_SNAPSHOT_FORMAT_VERSION {
            return Err(PreviewError::UnsupportedSnapshotFormat {
                snapshot_format_version: snapshot.snapshot_format_version(),
            });
        }
        if snapshot.state().asset_id() != &self.asset.header.asset_id {
            return Err(PreviewError::SnapshotAssetMismatch {
                expected: self.asset.header.asset_id.clone(),
                actual: snapshot.state().asset_id().clone(),
            });
        }
        let active_revision = PreviewAssetRevision::from_asset(self.asset).map_err(|error| {
            PreviewError::AssetRevisionFailed {
                reason: error.to_string(),
            }
        })?;
        if snapshot.asset_revision() != &active_revision {
            return Err(PreviewError::SnapshotStateMismatch);
        }
        if snapshot.initial_block().is_some_and(|block| {
            !self
                .asset
                .blocks
                .iter()
                .any(|candidate| candidate.id.as_str() == block)
        }) {
            return Err(PreviewError::SnapshotStateMismatch);
        }
        if snapshot.state().status().is_waiting_for_condition() {
            return Err(PreviewError::SnapshotPendingCondition);
        }
        let restored = restore_session(self.asset, snapshot.session().clone())
            .map_err(PreviewError::Runtime)?;
        if snapshot.options().locale().map(LocaleId::as_str)
            != restored.locale().map(LocaleId::as_str)
            || snapshot.state().locale().map(LocaleId::as_str)
                != restored.locale().map(LocaleId::as_str)
        {
            return Err(PreviewError::SnapshotStateMismatch);
        }
        if !state_matches_session(self.asset, snapshot.state(), &restored) {
            return Err(PreviewError::SnapshotStateMismatch);
        }
        self.session = restored;
        self.block = snapshot.initial_block.clone();
        self.options = snapshot.options.clone();
        self.next_condition_id = snapshot.next_condition_id;
        self.state = snapshot.state().clone();
        self.pending = None;
        self.restored_effect_reemit = match self.state.status() {
            PreviewStatus::WaitingForEffect { effect }
                if effect.mode == crate::DialogueEffectMode::Blocking =>
            {
                Some(effect.id.clone())
            }
            _ => None,
        };
        self.trace = super::model::PreviewTrace::new(
            self.session.locale().cloned(),
            self.options.variant.clone(),
        );
        self.transcript = super::model::PreviewTranscript::default();
        Ok(self.append_events(vec![PreviewEvent::Restored]))
    }
}
