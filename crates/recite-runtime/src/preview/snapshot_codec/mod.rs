//! Versioned persistence for the preview control contract.
//!
//! Event and transcript values remain runtime projections; only the stable
//! session/control state is encoded here.

mod line;
mod projection;
mod span;
mod wire;

#[cfg(test)]
mod tests;

use recite_core::{BlockId, ChoiceId, LocaleId};
use serde::Deserialize;
use std::io::Cursor;

use super::model::{
    PreviewConditionRequestId, PreviewError, PreviewSnapshot, PreviewState, PreviewStatus,
};
use wire::{SnapshotWire, StateWire, StatusWire};

impl PreviewSnapshot {
    /// Encodes only the versioned snapshot contract; preview events remain
    /// runtime values and are intentionally not part of this wire format.
    pub fn encode(&self) -> Result<Vec<u8>, PreviewError> {
        let wire = SnapshotWire::from_snapshot(self)?;
        rmp_serde::to_vec_named(&wire).map_err(|error| PreviewError::SnapshotEncodeFailed {
            reason: error.to_string(),
        })
    }

    /// Decodes a snapshot without accepting event or transcript wire shapes.
    pub fn decode(bytes: &[u8]) -> Result<Self, PreviewError> {
        let mut cursor = Cursor::new(bytes);
        let mut deserializer = rmp_serde::Deserializer::new(&mut cursor);
        let wire = SnapshotWire::deserialize(&mut deserializer).map_err(|error| {
            PreviewError::SnapshotDecodeFailed {
                reason: error.to_string(),
            }
        })?;
        if cursor.position() as usize != bytes.len() {
            return Err(PreviewError::SnapshotDecodeFailed {
                reason: format!(
                    "preview snapshot has {} trailing bytes",
                    bytes.len() - cursor.position() as usize
                ),
            });
        }
        if wire.version != super::model::PREVIEW_SNAPSHOT_FORMAT_VERSION {
            return Err(PreviewError::UnsupportedSnapshotFormat {
                snapshot_format_version: wire.version,
            });
        }
        wire.into_snapshot()
    }
}

impl SnapshotWire {
    fn from_snapshot(snapshot: &PreviewSnapshot) -> Result<Self, PreviewError> {
        Ok(Self {
            version: snapshot.snapshot_format_version(),
            session: snapshot.session().clone(),
            initial_block: snapshot.initial_block.clone(),
            locale: snapshot
                .options
                .locale()
                .map(|locale| locale.as_str().to_owned()),
            variant: snapshot.options.variant().map(str::to_owned),
            next_condition_id: snapshot.next_condition_request_id().get(),
            projection_fingerprint: snapshot.projection_fingerprint().to_owned(),
            state: StateWire::from_state(snapshot.state())?,
        })
    }

    fn into_snapshot(self) -> Result<PreviewSnapshot, PreviewError> {
        let locale = self
            .locale
            .map(LocaleId::new)
            .transpose()
            .map_err(invalid)?;
        let options = super::model::PreviewOptions::new()
            .with_optional_locale(locale)
            .with_optional_variant(self.variant);
        let state = self.state.into_state()?;
        Ok(PreviewSnapshot {
            snapshot_format_version: self.version,
            session: self.session,
            initial_block: self.initial_block,
            options,
            next_condition_id: PreviewConditionRequestId::new(self.next_condition_id),
            projection_fingerprint: self.projection_fingerprint,
            state,
        })
    }
}

impl StateWire {
    fn from_state(state: &PreviewState) -> Result<Self, PreviewError> {
        let status = match state.status() {
            PreviewStatus::Ready => StatusWire::Ready,
            PreviewStatus::WaitingForCondition { .. } => {
                return Err(PreviewError::SnapshotPendingCondition);
            }
            PreviewStatus::WaitingForChoice { prompt } => StatusWire::WaitingForChoice {
                prompt: Box::new(wire::PromptWire::from_prompt(prompt)),
            },
            PreviewStatus::WaitingForEffect { effect } => StatusWire::WaitingForEffect {
                effect: wire::EffectWire::from_effect(effect),
            },
            PreviewStatus::Ended => StatusWire::Ended,
        };
        Ok(Self {
            asset_id: state.asset_id().as_str().to_owned(),
            block: state.block().map(ToString::to_string),
            locale: state.locale().map(ToString::to_string),
            selected_choices: state
                .selected_choice_history()
                .iter()
                .map(ToString::to_string)
                .collect(),
            deferred_effects: state
                .deferred_effects()
                .iter()
                .map(wire::EffectWire::from_effect)
                .collect(),
            restart_required: state
                .restart_required()
                .map(wire::RequirementWire::from_requirement),
            status,
        })
    }

    fn into_state(self) -> Result<PreviewState, PreviewError> {
        let asset_id = recite_core::CompiledAssetId::new(self.asset_id).map_err(invalid)?;
        let block = self.block.map(BlockId::new).transpose().map_err(invalid)?;
        let locale = self
            .locale
            .map(LocaleId::new)
            .transpose()
            .map_err(invalid)?;
        let selected = self
            .selected_choices
            .into_iter()
            .map(ChoiceId::new)
            .collect::<Result<Vec<_>, _>>()
            .map_err(invalid)?;
        let deferred = self
            .deferred_effects
            .into_iter()
            .map(wire::EffectWire::into_effect)
            .collect::<Result<Vec<_>, _>>()?;
        let restart_required = self
            .restart_required
            .map(wire::RequirementWire::into_requirement)
            .transpose()?;
        let status = match self.status {
            StatusWire::Ready => PreviewStatus::Ready,
            StatusWire::WaitingForChoice { prompt } => PreviewStatus::WaitingForChoice {
                prompt: Box::new(prompt.into_prompt()?),
            },
            StatusWire::WaitingForEffect { effect } => PreviewStatus::WaitingForEffect {
                effect: effect.into_effect()?,
            },
            StatusWire::Ended => PreviewStatus::Ended,
        };
        Ok(PreviewState::from_parts(
            asset_id,
            block,
            locale,
            status,
            selected,
            deferred,
            restart_required,
        ))
    }
}

fn invalid(error: impl std::fmt::Display) -> PreviewError {
    PreviewError::SnapshotDecodeFailed {
        reason: error.to_string(),
    }
}
