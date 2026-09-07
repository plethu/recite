use recite_compiler::{CompileInput, CompileOptions, compile_inputs};
use recite_core::{
    ChoiceId, CompiledAssetId, CompiledDialogue, CompilerVersion, SchemaFingerprint, SourceMapId,
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
    #[error(
        "This experiment stops at condition or effect requests; it never executes game effects."
    )]
    UnsupportedRequest,
}

#[derive(Clone, Debug, Default)]
pub struct PreviewPage {
    pub text: String,
    pub choices: Vec<DialogueChoice>,
    pub ended: bool,
}

/// The compiled asset remains fixed until an explicit new preview is started.
pub struct Preview {
    asset: CompiledDialogue,
    state: Option<PreviewSnapshot>,
    revision: i64,
}

impl Preview {
    pub fn new(document: &Document) -> Result<Self, PreviewError> {
        let report = compile_inputs(
            [CompileInput::new(
                document.key().as_str(),
                document.source(),
            )],
            CompileOptions::new(
                CompilerVersion::new("0.1.0")?,
                CompiledAssetId::new("bakeoff/scene.recitec")?,
                SourceMapId::new("bakeoff/scene.map")?,
                SchemaFingerprint::NoSchema,
            ),
        )?;
        let asset = report.asset.ok_or(PreviewError::InvalidSource)?.dialogue;
        Ok(Self {
            asset,
            state: None,
            revision: document.revision(),
        })
    }

    pub const fn revision(&self) -> i64 {
        self.revision
    }

    pub fn advance(&mut self, choice: Option<ChoiceId>) -> Result<PreviewPage, PreviewError> {
        let mut session = PreviewSession::new(&self.asset, None, PreviewOptions::new())?;
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
                PreviewEvent::Error(error) => return Err(error.clone().into()),
                _ => return Err(PreviewError::UnsupportedRequest),
            }
        }
        self.state = Some(session.snapshot()?);
        Ok(page)
    }
}
