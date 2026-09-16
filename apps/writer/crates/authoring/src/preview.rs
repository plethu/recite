use recite_compiler::{CompileInput, CompileOptions, compile_inputs, compile_inputs_with_schema};
use recite_core::{
    ChoiceId, CompiledAssetId, CompiledDialogue, CompilerVersion, ProjectSchema, SchemaFingerprint,
    SourceMapId,
};
use recite_runtime::{
    DialogueChoice, PreviewEvent, PreviewInputs, PreviewOptions, PreviewSession, PreviewSnapshot,
};

use crate::Document;

#[derive(Debug, thiserror::Error)]
pub enum PreviewError {
    #[error(transparent)]
    Value(#[from] recite_core::CompiledValueError),
    #[error(transparent)]
    Compile(#[from] recite_compiler::CompileError),
    #[error(transparent)]
    Runtime(#[from] recite_runtime::DialogueError),
    #[error(transparent)]
    Preview(#[from] recite_runtime::PreviewError),
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
}

/// The compiled asset remains fixed until an explicit new preview is started.
pub struct Preview {
    asset: CompiledDialogue,
    state: Option<PreviewSnapshot>,
    revision: i64,
    entry: Option<String>,
}

impl Preview {
    pub fn new(document: &Document) -> Result<Self, PreviewError> {
        Self::at_block(document, None)
    }

    pub fn at_block(document: &Document, block: Option<&str>) -> Result<Self, PreviewError> {
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
            asset,
            state: None,
            revision: document.revision(),
            entry: block.map(str::to_owned),
        })
    }

    pub const fn revision(&self) -> i64 {
        self.revision
    }

    pub fn advance(&mut self, choice: Option<ChoiceId>) -> Result<PreviewPage, PreviewError> {
        let mut session =
            PreviewSession::new(&self.asset, self.entry.as_deref(), PreviewOptions::new())?;
        if let Some(snapshot) = &self.state {
            session.restore(snapshot.clone())?;
        }
        let output = if let Some(choice) = choice {
            session.choose(choice, PreviewInputs::new())
        } else {
            session.step(PreviewInputs::new())
        };
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
                PreviewEvent::End { .. } => {
                    page.text = "End conversation".to_owned();
                    page.ended = true;
                }
                PreviewEvent::ChoiceSelected { .. } => {}
                PreviewEvent::EffectRequested(effect)
                | PreviewEvent::DeferredEffectScheduled(effect) => {
                    page.text = "Effect requested".into();
                    page.effects.push(effect.clone());
                }
                PreviewEvent::Error(error) => return Err(error.clone().into()),
                _ => return Err(PreviewError::UnsupportedRequest),
            }
        }
        self.state = Some(session.snapshot()?);
        Ok(page)
    }
}
