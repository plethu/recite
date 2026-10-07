//! Private browser host for the production compiler and runtime.
//!
//! Owns one in-memory source file and one session. Game queries are deliberately
//! unregistered; effects remain requests, with explicit simulated acknowledgement.
//! The site runs this bridge in a terminable Worker, never on the UI thread.

mod output;

use recite_compiler::compile::{CompileInput, CompileOptions, compile_inputs};
use recite_core::{
    ChoiceId,
    compiled::{
        CompiledAssetId, CompiledDialogue, CompilerVersion, SchemaFingerprint, SourceMapId,
    },
};
use recite_runtime::{
    DialogueSession, EffectAck, EmptyDialogueContext, acknowledge_effect, choose, next, start_scene,
};
use wasm_bindgen::prelude::*;

use output::BrowserOutput;

const MAX_SOURCE_BYTES: usize = 64 * 1024;

struct ActiveDialogue {
    asset: CompiledDialogue,
    session: DialogueSession,
}

/// One browser-owned dialogue; editing and running starts a fresh session.
#[wasm_bindgen]
#[derive(Default)]
pub struct Playground {
    active: Option<ActiveDialogue>,
}

#[wasm_bindgen]
impl Playground {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Compile actual Recite source and return its first runtime event or diagnostics.
    pub fn run(&mut self, source: &str) -> Result<String, String> {
        self.active = None;
        if source.len() > MAX_SOURCE_BYTES {
            return Err("This preview accepts one source file up to 64 KiB.".to_owned());
        }
        let options = CompileOptions::new(
            CompilerVersion::new(env!("CARGO_PKG_VERSION")).map_err(|error| error.to_string())?,
            CompiledAssetId::new("browser-preview").map_err(|error| error.to_string())?,
            SourceMapId::new("browser-preview-source").map_err(|error| error.to_string())?,
            SchemaFingerprint::NoSchema,
        );
        let report = compile_inputs([CompileInput::new("example.recite", source)], options)
            .map_err(|error| error.to_string())?;
        let Some(asset) = report.asset else {
            return output::encode(BrowserOutput::diagnostics(report.diagnostics)?);
        };
        let session = start_scene(&asset.dialogue, None).map_err(|error| error.to_string())?;
        self.active = Some(ActiveDialogue {
            asset: asset.dialogue,
            session,
        });
        self.advance()
    }

    /// Advance one event; the runtime owns traversal and its silent-step budget.
    pub fn advance(&mut self) -> Result<String, String> {
        let active = self.active.as_mut().ok_or("Run a scene first.")?;
        let event = next(&active.asset, &mut active.session, &EmptyDialogueContext)
            .map_err(|error| error.to_string())?;
        output::encode(BrowserOutput::from(event))
    }

    /// Select the stable ID from the currently displayed runtime prompt.
    pub fn select(&mut self, id: &str) -> Result<String, String> {
        let choice = ChoiceId::new(id).map_err(|error| error.to_string())?;
        let active = self.active.as_mut().ok_or("Run a scene first.")?;
        let event = choose(
            &active.asset,
            &mut active.session,
            choice,
            &EmptyDialogueContext,
        )
        .map_err(|error| error.to_string())?;
        output::encode(BrowserOutput::from(event))
    }

    /// Simulate completion of the pending blocking request without running an effect.
    pub fn acknowledge(&mut self) -> Result<String, String> {
        let active = self.active.as_mut().ok_or("Run a scene first.")?;
        let id = active
            .session
            .pending_effect()
            .ok_or("No blocking effect is pending.")?
            .id
            .clone();
        acknowledge_effect(&mut active.session, id, EffectAck::Completed)
            .map_err(|error| error.to_string())?;
        self.advance()
    }
}
