//! Growing session workloads; setup and snapshot restoration are not timed.

use recite_adapter::{LoadedDialogue, SessionDriver, StartRequest};
use recite_compiler::compile::{CompileInput, CompileOptions, compile_inputs};
use recite_core::{
    ChoiceId,
    compiled::{CompiledAssetId, CompilerVersion, SchemaFingerprint, SourceMapId},
};
use recite_runtime::{
    ConditionEvaluationError, ConditionQuery, ConditionValue, DialogueEvent,
    DialogueSessionOptions, LocaleResolution,
    preview::{
        ConditionAnswer, PreviewEvent, PreviewInputs, PreviewOptions, PreviewOutput,
        PreviewSession, PreviewSnapshot,
    },
};

pub(super) type SessionResult<T> = Result<T, Box<dyn std::error::Error>>;

fn error(message: impl Into<String>) -> Box<dyn std::error::Error> {
    message.into().into()
}

pub(super) const SESSION_HISTORY_SIZES: [usize; 4] = [0, 32, 256, 2_048];

pub(super) struct SessionWorkload {
    asset: LoadedDialogue,
    choice: ChoiceId,
}

impl SessionWorkload {
    pub(super) fn new(deferred_effects: bool) -> SessionResult<Self> {
        let source = if deferred_effects {
            include_str!("../../../../fixtures/recite/valid/benchmarks/session_deferred.recite")
        } else {
            include_str!("../../../../fixtures/recite/valid/benchmarks/session_history.recite")
        };
        let report = compile_inputs(
            [CompileInput::new("session.recite", source)],
            CompileOptions::new(
                CompilerVersion::new("benchmarks")?,
                CompiledAssetId::new("session.recitec")?,
                SourceMapId::new("session.recitec.map")?,
                SchemaFingerprint::NoSchema,
            ),
        )?;
        if !report.diagnostics.is_empty() {
            return Err(error(format!(
                "session fixture diagnostics: {:?}",
                report.diagnostics
            )));
        }
        let compiled = report.asset.ok_or_else(|| error("missing session asset"))?;
        Ok(Self {
            asset: LoadedDialogue::from_bytes(&compiled.messagepack)?,
            choice: ChoiceId::new("11111111111111111111")?,
        })
    }

    pub(super) fn adapter_checkpoint(&self, history: usize) -> SessionResult<Vec<u8>> {
        let mut driver = SessionDriver::new();
        driver.start(
            StartRequest {
                asset: &self.asset,
                block_id: None,
                options: DialogueSessionOptions::new(),
            },
            &ready,
            LocaleResolution::new(),
        )?;
        for _ in 0..history {
            self.adapter_cycle(&mut driver)?;
        }
        Ok(driver.snapshot()?)
    }

    pub(super) fn restore_adapter(&self, checkpoint: &[u8]) -> SessionResult<SessionDriver> {
        let mut driver = SessionDriver::new();
        driver.prepare_restore(&self.asset, checkpoint)?;
        driver.begin(&ready, LocaleResolution::new())?;
        Ok(driver)
    }

    pub(super) fn adapter_cycle(
        &self,
        driver: &mut SessionDriver,
    ) -> SessionResult<Vec<DialogueEvent>> {
        Ok(driver.select_choice(self.choice.clone(), &ready, LocaleResolution::new())?)
    }

    pub(super) fn preview_checkpoint(&self, history: usize) -> SessionResult<PreviewSnapshot> {
        let mut preview = PreviewSession::new(self.asset.dialogue(), None, PreviewOptions::new())?;
        let initial = preview.step(PreviewInputs::new());
        to_prompt(&mut preview, initial)?;
        for _ in 0..history {
            self.preview_cycle(&mut preview)?;
        }
        Ok(preview.snapshot()?)
    }

    pub(super) fn restore_preview(
        &self,
        checkpoint: &PreviewSnapshot,
    ) -> SessionResult<PreviewSession<'_>> {
        let mut preview = PreviewSession::new(self.asset.dialogue(), None, PreviewOptions::new())?;
        preview.restore(checkpoint.clone())?;
        Ok(preview)
    }

    pub(super) fn preview_cycle(
        &self,
        preview: &mut PreviewSession<'_>,
    ) -> SessionResult<PreviewOutput> {
        let output = preview.choose(self.choice.clone(), PreviewInputs::new());
        to_prompt(preview, output)
    }
}

fn ready(query: ConditionQuery<'_>) -> Result<ConditionValue, ConditionEvaluationError> {
    if query.function() == "ready" {
        Ok(ConditionValue::Bool(true))
    } else {
        Err(ConditionEvaluationError::new(
            "unexpected benchmark condition",
        ))
    }
}

fn to_prompt(
    preview: &mut PreviewSession<'_>,
    mut output: PreviewOutput,
) -> SessionResult<PreviewOutput> {
    // This fixture has one condition, one optional deferred effect and one
    // choice echo per cycle. Fail instead of hanging if its shape changes.
    for _ in 0..8 {
        let mut condition = None;
        for event in output.events() {
            match event {
                PreviewEvent::Error(failure) => return Err(error(failure.to_string())),
                PreviewEvent::Prompt(_) => return Ok(output),
                PreviewEvent::ConditionRequested(request) => {
                    if request.query().function() != "ready" {
                        return Err(error("unexpected preview condition"));
                    }
                    condition = Some(request.id());
                }
                PreviewEvent::End { .. } => return Err(error("session fixture ended")),
                _ => {}
            }
        }
        output = match condition {
            Some(id) => preview.answer(
                id,
                ConditionAnswer::Value(ConditionValue::Bool(true)),
                PreviewInputs::new(),
            ),
            None => preview.step(PreviewInputs::new()),
        };
    }
    Err(error("session fixture did not return to its prompt"))
}
