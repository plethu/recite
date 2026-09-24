mod catalogue;
mod setup;
pub use setup::PreviewSetup;

use recite_compiler::compile::{
    CompileInput, CompileOptions, compile_inputs, compile_inputs_with_schema,
};
use recite_core::{
    ChoiceId,
    compiled::{
        CompiledAssetId, CompiledDialogue, CompilerVersion, SchemaFingerprint, SourceMapId,
    },
    schema::ProjectSchema,
};
use recite_runtime::{
    ConditionValue, DialogueChoice, DialogueEffectMode, EffectAck,
    preview::{
        ConditionAnswer, PreviewCommand, PreviewConditionRequest, PreviewEvent, PreviewInputs,
        PreviewSession,
    },
};

use crate::Document;

#[derive(Debug, thiserror::Error)]
pub enum PreviewError {
    #[error(transparent)]
    Value(#[from] recite_core::compiled::CompiledValueError),
    #[error(transparent)]
    Compile(#[from] recite_compiler::compile::CompileError),
    #[error(transparent)]
    Runtime(#[from] recite_runtime::DialogueError),
    #[error(transparent)]
    Preview(#[from] recite_runtime::preview::PreviewError),
    #[error(transparent)]
    Locale(#[from] recite_runtime::localisation::LocaleError),
    #[error("Repair the scene diagnostics before starting a new preview.")]
    InvalidSource,
    #[error("Preview needs condition input before it can continue.")]
    UnsupportedRequest,
}

#[derive(Clone, Debug, Default)]
pub struct PreviewPage {
    pub text: String,
    pub choices: Vec<DialogueChoice>,
    pub ended: bool,
    pub effects: Vec<recite_runtime::DialogueEffectRequest>,
    pub condition: Option<PreviewConditionRequest>,
    pub waiting_effect: Option<recite_core::EffectId>,
}

/// The compiled asset remains fixed until an explicit new preview is started.
pub struct Preview {
    session: OwnedPreview,
    revision: i64,
    catalogues: catalogue::TrialCatalogues,
    values: recite_runtime::localisation::InterpolationValues,
}

self_cell::self_cell! {
    struct OwnedPreview {
        owner: CompiledDialogue,
        #[covariant]
        dependent: PreviewSession,
    }
}

impl Preview {
    pub fn new(document: &Document) -> Result<Self, PreviewError> {
        Self::at_block(document, None)
    }

    pub fn at_block(document: &Document, block: Option<&str>) -> Result<Self, PreviewError> {
        Self::configured(document, block, PreviewSetup::default())
    }

    pub fn configured(
        document: &Document,
        block: Option<&str>,
        setup: PreviewSetup,
    ) -> Result<Self, PreviewError> {
        let trial_options = setup.options();
        let expected = document
            .extract_catalogue()
            .catalog
            .ok_or(PreviewError::InvalidSource)?;
        let catalogues =
            catalogue::TrialCatalogues::new(setup.catalogues, &setup.policy, &expected)?;
        let snapshot = document.kernel().snapshot();
        let inputs = snapshot
            .documents()
            .iter()
            .map(|input| CompileInput::new(input.key().as_str(), input.source_text()));
        let options = CompileOptions::new(
            CompilerVersion::new("0.1.0")?,
            CompiledAssetId::new("workbench/preview.recitec")?,
            SourceMapId::new("workbench/preview.map")?,
            document.schema().map_or(
                SchemaFingerprint::NoSchema,
                ProjectSchema::canonical_fingerprint,
            ),
        );
        let report = match document.schema() {
            Some(schema) => compile_inputs_with_schema(inputs, options, schema)?,
            None => compile_inputs(inputs, options)?,
        };
        let asset = report.asset.ok_or(PreviewError::InvalidSource)?.dialogue;
        Ok(Self {
            session: OwnedPreview::try_new(asset, |asset| {
                PreviewSession::new(asset, block, trial_options)
            })?,
            revision: document.revision(),
            catalogues,
            values: setup.values,
        })
    }

    pub fn trace(&self) -> &recite_runtime::preview::PreviewTrace {
        self.session.borrow_dependent().trace()
    }

    pub fn events(&self) -> &[PreviewEvent] {
        self.session.borrow_dependent().trace().events()
    }

    pub const fn revision(&self) -> i64 {
        self.revision
    }

    pub fn advance(&mut self, choice: Option<ChoiceId>) -> Result<PreviewPage, PreviewError> {
        self.dispatch(choice.map_or(PreviewCommand::Advance, |choice_id| {
            PreviewCommand::Choose { choice_id }
        }))
    }

    pub fn answer(
        &mut self,
        request: &PreviewConditionRequest,
        value: ConditionValue,
    ) -> Result<PreviewPage, PreviewError> {
        self.dispatch(PreviewCommand::Answer {
            request_id: request.id(),
            answer: ConditionAnswer::Value(value),
        })
    }

    pub fn acknowledge(
        &mut self,
        id: recite_core::EffectId,
        ack: EffectAck,
    ) -> Result<PreviewPage, PreviewError> {
        self.dispatch(PreviewCommand::Acknowledge { effect_id: id, ack })
    }

    fn dispatch(&mut self, command: PreviewCommand) -> Result<PreviewPage, PreviewError> {
        let inputs = PreviewInputs::new()
            .with_locale_provider(&self.catalogues)
            .with_interpolation_values(&self.values);
        let output = self
            .session
            .with_dependent_mut(|_, session| session.dispatch(command, inputs));
        let mut page = PreviewPage::default();
        for event in output.events() {
            match event {
                PreviewEvent::Line(line) => page.text = line.text.clone(),
                PreviewEvent::Prompt(prompt) => {
                    page.text = prompt
                        .line()
                        .map_or_else(|| "Choose a reply".to_owned(), |line| line.text.clone());
                    page.choices = prompt.choices().to_vec();
                }
                PreviewEvent::End { deferred_effects } => {
                    page.effects.clone_from(deferred_effects);
                    page.text = "End conversation".to_owned();
                    page.ended = true;
                }
                PreviewEvent::ChoiceSelected { .. }
                | PreviewEvent::ChoiceAccepted { .. }
                | PreviewEvent::ConditionResult { .. }
                | PreviewEvent::EffectAcknowledged { .. } => {}
                PreviewEvent::ConditionRequested(request) => {
                    page.condition = Some(request.clone());
                }
                PreviewEvent::EffectRequested(effect)
                | PreviewEvent::DeferredEffectScheduled(effect) => {
                    page.text = "Effect requested".into();
                    page.effects.push(effect.clone());
                    if effect.mode == DialogueEffectMode::Blocking {
                        page.waiting_effect = Some(effect.id.clone());
                    }
                }
                PreviewEvent::Error(error) => return Err(error.clone().into()),
                _ => return Err(PreviewError::UnsupportedRequest),
            }
        }
        Ok(page)
    }
}
