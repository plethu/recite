use recite_core::{ChoiceId, EffectId, LineId};

use super::super::model::PreviewError;
use super::super::model::{
    PreviewAssetRevision, PreviewPrompt, PreviewPromptIdentity, PreviewRestartRequirement,
};
use super::span::SpanWire;
use super::wire::{
    ArgumentWire, AssetRevisionWire, ChoiceWire, DialogueEffectModeWire, EffectWire, LineWire,
    PromptWire, RequirementWire,
};
use crate::{
    ChoiceEchoMode, DialogueChoice, DialogueEffectArgument, DialogueEffectMode,
    DialogueEffectRequest,
};

impl PromptWire {
    pub(super) fn from_prompt(prompt: &PreviewPrompt) -> Self {
        Self {
            block: prompt.identity().block().to_string(),
            line: prompt.identity().line().map(ToString::to_string),
            choices: prompt
                .identity()
                .choices()
                .iter()
                .map(ToString::to_string)
                .collect(),
            line_projection: prompt.line().map(LineWire::from_line),
            choice_projection: prompt
                .choices()
                .iter()
                .map(ChoiceWire::from_choice)
                .collect(),
        }
    }

    pub(super) fn into_prompt(self) -> Result<PreviewPrompt, PreviewError> {
        let identity = PreviewPromptIdentity::from_parts(
            recite_core::BlockId::new(self.block).map_err(invalid)?,
            self.line.map(LineId::new).transpose().map_err(invalid)?,
            self.choices
                .into_iter()
                .map(ChoiceId::new)
                .collect::<Result<Vec<_>, _>>()
                .map_err(invalid)?,
        );
        let line = self.line_projection.map(LineWire::into_line).transpose()?;
        let choices = self
            .choice_projection
            .into_iter()
            .map(ChoiceWire::into_choice)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(PreviewPrompt::from_parts(identity, line, choices))
    }
}

impl ChoiceWire {
    fn from_choice(choice: &DialogueChoice) -> Self {
        Self {
            id: choice.id.to_string(),
            source_text: choice.source_text.clone(),
            text: choice.text.clone(),
            availability: crate::session_snapshot::availability_snapshot(&choice.availability),
            metadata: choice
                .metadata
                .iter()
                .map(super::wire::MetadataWire::from_entry)
                .collect(),
            echo: match &choice.echo {
                ChoiceEchoMode::None => super::wire::EchoWire::None,
                ChoiceEchoMode::SelectedText => super::wire::EchoWire::SelectedText,
                ChoiceEchoMode::ExplicitLine(id) => {
                    super::wire::EchoWire::ExplicitLine(id.to_string())
                }
            },
        }
    }

    fn into_choice(self) -> Result<DialogueChoice, PreviewError> {
        let availability = crate::session_snapshot::availability_from_snapshot(self.availability)
            .map_err(|error| invalid(error.to_string()))?;
        Ok(DialogueChoice {
            id: ChoiceId::new(self.id).map_err(invalid)?,
            source_text: self.source_text,
            text: self.text,
            metadata: self
                .metadata
                .into_iter()
                .map(super::wire::MetadataWire::into_entry)
                .collect::<Result<Vec<_>, _>>()?,
            availability,
            echo: match self.echo {
                super::wire::EchoWire::None => ChoiceEchoMode::None,
                super::wire::EchoWire::SelectedText => ChoiceEchoMode::SelectedText,
                super::wire::EchoWire::ExplicitLine(id) => {
                    ChoiceEchoMode::ExplicitLine(LineId::new(id).map_err(invalid)?)
                }
            },
        })
    }
}

impl EffectWire {
    pub(super) fn from_effect(effect: &DialogueEffectRequest) -> Self {
        Self {
            id: effect.id.to_string(),
            mode: effect.mode.into(),
            function: effect.function.clone(),
            args: effect
                .args
                .iter()
                .map(ArgumentWire::from_argument)
                .collect(),
            source_span: SpanWire::from(&effect.source_span),
        }
    }

    pub(super) fn into_effect(self) -> Result<DialogueEffectRequest, PreviewError> {
        Ok(DialogueEffectRequest {
            id: EffectId::new(self.id).map_err(invalid)?,
            mode: self.mode.into(),
            function: self.function,
            args: self
                .args
                .into_iter()
                .map(ArgumentWire::into_argument)
                .collect(),
            source_span: self.source_span.into_span()?,
        })
    }
}

impl ArgumentWire {
    fn from_argument(argument: &DialogueEffectArgument) -> Self {
        match argument {
            DialogueEffectArgument::Identifier(value) => Self::Identifier(value.clone()),
            DialogueEffectArgument::String(value) => Self::String(value.clone()),
            DialogueEffectArgument::Integer(value) => Self::Integer(*value),
            DialogueEffectArgument::Float(value) => Self::Float(*value),
            DialogueEffectArgument::Boolean(value) => Self::Boolean(*value),
        }
    }

    fn into_argument(self) -> DialogueEffectArgument {
        match self {
            Self::Identifier(value) => DialogueEffectArgument::Identifier(value),
            Self::String(value) => DialogueEffectArgument::String(value),
            Self::Integer(value) => DialogueEffectArgument::Integer(value),
            Self::Float(value) => DialogueEffectArgument::Float(value),
            Self::Boolean(value) => DialogueEffectArgument::Boolean(value),
        }
    }
}

impl From<DialogueEffectMode> for DialogueEffectModeWire {
    fn from(mode: DialogueEffectMode) -> Self {
        match mode {
            DialogueEffectMode::Deferred => Self::Deferred,
            DialogueEffectMode::Immediate => Self::Immediate,
            DialogueEffectMode::Blocking => Self::Blocking,
        }
    }
}

impl From<DialogueEffectModeWire> for DialogueEffectMode {
    fn from(mode: DialogueEffectModeWire) -> Self {
        match mode {
            DialogueEffectModeWire::Deferred => Self::Deferred,
            DialogueEffectModeWire::Immediate => Self::Immediate,
            DialogueEffectModeWire::Blocking => Self::Blocking,
        }
    }
}

impl RequirementWire {
    pub(super) fn from_requirement(requirement: &PreviewRestartRequirement) -> Self {
        Self {
            active_asset: requirement.active_asset().as_str().to_owned(),
            replacement_asset: requirement.replacement_asset().as_str().to_owned(),
            active_revision: Some(AssetRevisionWire::from_revision(
                requirement.active_revision(),
            )),
            replacement_revision: Some(AssetRevisionWire::from_revision(
                requirement.replacement_revision(),
            )),
        }
    }

    pub(super) fn into_requirement(self) -> Result<PreviewRestartRequirement, PreviewError> {
        let active_asset = recite_core::CompiledAssetId::new(self.active_asset).map_err(invalid)?;
        let replacement_asset =
            recite_core::CompiledAssetId::new(self.replacement_asset).map_err(invalid)?;
        let active_revision = self
            .active_revision
            .ok_or_else(|| invalid("restart requirement is missing active revision"))?
            .into_revision()?;
        let replacement_revision = self
            .replacement_revision
            .ok_or_else(|| invalid("restart requirement is missing replacement revision"))?
            .into_revision()?;
        if active_revision.asset_id() != &active_asset
            || replacement_revision.asset_id() != &replacement_asset
        {
            return Err(invalid("restart requirement revision asset ID mismatch"));
        }
        Ok(PreviewRestartRequirement::new(
            active_asset,
            replacement_asset,
            active_revision,
            replacement_revision,
        ))
    }
}

impl AssetRevisionWire {
    fn from_revision(revision: &PreviewAssetRevision) -> Self {
        Self {
            asset_id: revision.asset_id().as_str().to_owned(),
            payload_fingerprint: revision.fingerprint_snapshot(),
        }
    }

    fn into_revision(self) -> Result<PreviewAssetRevision, PreviewError> {
        PreviewAssetRevision::from_fingerprint_snapshot(
            recite_core::CompiledAssetId::new(self.asset_id).map_err(invalid)?,
            self.payload_fingerprint,
        )
    }
}

fn invalid(error: impl std::fmt::Display) -> PreviewError {
    PreviewError::SnapshotDecodeFailed {
        reason: error.to_string(),
    }
}
