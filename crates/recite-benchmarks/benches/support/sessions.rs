//! Growing session workloads; setup and snapshot restoration are not timed.

use recite_adapter::{LoadedDialogue, SessionDriver, StartRequest};
use recite_core::ChoiceId;
use recite_runtime::{
    ConditionValue, DialogueEvent, DialogueSessionOptions, LocaleResolution,
    preview::{
        ConditionAnswer, PreviewEvent, PreviewInputs, PreviewOptions, PreviewOutput,
        PreviewSession, PreviewSnapshot,
    },
};

#[path = "session_fixture.rs"]
mod session_fixture;
use session_fixture::ready;

pub(super) type SessionResult<T> = Result<T, Box<dyn std::error::Error>>;

fn error(message: impl Into<String>) -> Box<dyn std::error::Error> {
    message.into().into()
}

pub(super) const SESSION_HISTORY_SIZES: [usize; 4] = [0, 32, 256, 2_048];
pub(super) const SESSION_SOURCE_COUNTS: [usize; 4] = [1, 40, 400, 4_000];

pub(super) struct SessionWorkload {
    asset: LoadedDialogue,
    choice: ChoiceId,
}

impl SessionWorkload {
    pub(super) fn new(deferred_effects: bool) -> SessionResult<Self> {
        Self::with_source_count(deferred_effects, 1)
    }

    pub(super) fn with_source_count(
        deferred_effects: bool,
        source_count: usize,
    ) -> SessionResult<Self> {
        let (asset, choice) = session_fixture::load(deferred_effects, source_count)?;
        Ok(Self { asset, choice })
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
