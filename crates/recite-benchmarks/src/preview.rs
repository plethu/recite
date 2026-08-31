use recite_core::{CompiledDialogue, EffectId};
use recite_runtime::{
    ConditionAnswer, PreviewEvent, PreviewInputs, PreviewOptions, PreviewPrompt, PreviewSession,
    PreviewSnapshot, PreviewStatus,
};

use crate::compiler::CompilerProject;
use crate::fixture_context::RuntimeFixture;
use crate::project::BenchmarkProject;
use crate::{BenchmarkFixture, BenchmarkResult, error};

pub use crate::preview_retention::{
    PreviewRetentionReport, PreviewSnapshotShape, PreviewTraceShape,
};

/// A compiled, host-neutral project prepared for preview benchmarks.
///
/// The benchmark driver answers preview condition requests from the same
/// fixture data used by the ordinary runtime benchmarks and selects the
/// fixture's stable choice IDs. It deliberately does not add editor or engine
/// behavior to the preview surface.
#[derive(Clone, Debug)]
pub struct PreviewProject {
    fixture: BenchmarkFixture,
    asset: CompiledDialogue,
    runtime_fixture: RuntimeFixture,
    catalog: crate::catalog::CatalogProvider,
}

impl PreviewProject {
    pub fn load(fixture: BenchmarkFixture) -> BenchmarkResult<Self> {
        let project = BenchmarkProject::load_fixture(fixture)?;
        let compiler = CompilerProject::load(&project)?;
        let compiled = compiler.compile_with_schema()?;
        let runtime_fixture = RuntimeFixture::load(&project.runtime_fixture_source()?)?;
        let catalog = crate::catalog::CatalogProvider::load(&project, &runtime_fixture)?;
        Ok(Self {
            fixture,
            asset: compiled.asset().dialogue.clone(),
            runtime_fixture,
            catalog,
        })
    }

    #[must_use]
    pub fn fixture(&self) -> BenchmarkFixture {
        self.fixture
    }

    #[must_use]
    pub fn fixture_label(&self) -> &'static str {
        self.fixture.as_str()
    }

    pub fn start(&self) -> BenchmarkResult<PreviewSession<'_>> {
        let options = PreviewOptions::new().with_locale(self.runtime_fixture.locale());
        PreviewSession::new(&self.asset, None, options).map_err(Into::into)
    }

    /// Prepares a preview at its first prompt without selecting a choice.
    ///
    /// This is the useful retained-state boundary for snapshot evidence: it
    /// includes the initial effects and condition-driven prompt projection,
    /// while remaining resumable through the public preview API.
    pub fn at_first_prompt(&self) -> BenchmarkResult<PreviewSession<'_>> {
        let mut preview = self.start()?;
        let mut output = preview.step(self.inputs());
        loop {
            let mut progressed = false;
            for event in output.events() {
                match event {
                    PreviewEvent::ConditionRequested(request) => {
                        let answer = self
                            .runtime_fixture
                            .preview_condition_answer(request.query())?;
                        output = answer_condition(&mut preview, request.id(), answer, self)?;
                        progressed = true;
                        break;
                    }
                    PreviewEvent::EffectRequested(effect)
                        if effect.mode == recite_runtime::DialogueEffectMode::Blocking =>
                    {
                        output = acknowledge_blocking(&mut preview, effect.id.clone())?;
                        progressed = true;
                        break;
                    }
                    PreviewEvent::Prompt(_) => return Ok(preview),
                    PreviewEvent::End { .. } => {
                        return Err(error("preview fixture ended before its first prompt"));
                    }
                    PreviewEvent::Error(preview_error) => {
                        return Err(error(format!("preview driver event: {preview_error}")));
                    }
                    _ => {}
                }
            }
            if !progressed {
                output = preview.step(self.inputs());
            }
        }
    }

    /// Drives a preview from its current boundary to `End`, answering
    /// conditions and selecting fixture-declared stable choices.
    pub fn collect_to_end(
        &self,
        preview: &mut PreviewSession<'_>,
    ) -> BenchmarkResult<Vec<PreviewEvent>> {
        let mut events = Vec::new();
        let mut output = initial_output(preview, self)?;
        loop {
            events.extend(output.events().iter().cloned());
            let mut progressed = false;
            for event in output.events() {
                match event {
                    PreviewEvent::ConditionRequested(request) => {
                        let answer = self
                            .runtime_fixture
                            .preview_condition_answer(request.query())?;
                        output = answer_condition(preview, request.id(), answer, self)?;
                        progressed = true;
                        break;
                    }
                    PreviewEvent::Prompt(prompt) => {
                        let choice_id = self.choice_for_prompt(prompt)?;
                        output = preview.choose(choice_id, self.inputs());
                        progressed = true;
                        break;
                    }
                    PreviewEvent::EffectRequested(effect)
                        if effect.mode == recite_runtime::DialogueEffectMode::Blocking =>
                    {
                        output = acknowledge_blocking(preview, effect.id.clone())?;
                        progressed = true;
                        break;
                    }
                    PreviewEvent::End { .. } => return Ok(events),
                    PreviewEvent::Error(preview_error) => {
                        return Err(error(format!("preview driver event: {preview_error}")));
                    }
                    _ => {}
                }
            }
            if !progressed {
                output = preview.step(self.inputs());
            }
        }
    }

    pub fn full_traversal(&self) -> BenchmarkResult<usize> {
        let mut preview = self.start()?;
        Ok(self.collect_to_end(&mut preview)?.len())
    }

    /// Captures the exact encoded snapshot size and deterministic retained
    /// trace/transcript shape at the supplied preview boundary.
    pub fn retention_report(
        &self,
        preview: &PreviewSession<'_>,
    ) -> BenchmarkResult<PreviewRetentionReport> {
        crate::preview_retention::build_report(self.fixture_label(), preview)
    }

    /// Restores a prompt snapshot into a fresh session and compares all future
    /// externally visible events through the end of the scene.
    pub fn restore_parity(&self) -> BenchmarkResult<PreviewRestoreParity> {
        let mut original = self.at_first_prompt()?;
        let snapshot = original.snapshot().map_err(preview_error)?;
        let encoded = snapshot.encode().map_err(preview_error)?;
        let decoded = PreviewSnapshot::decode(&encoded).map_err(preview_error)?;
        let mut restored = self.start()?;
        restored.restore(decoded).map_err(preview_error)?;

        let original_events = self.collect_to_end(&mut original)?;
        let restored_events = self.collect_to_end(&mut restored)?;
        Ok(PreviewRestoreParity {
            events_match: original_events == restored_events,
            original_event_count: original_events.len(),
            restored_event_count: restored_events.len(),
        })
    }

    pub fn inputs(&self) -> PreviewInputs<'_> {
        PreviewInputs::new().with_locale_provider(&self.catalog)
    }

    fn choice_for_prompt(&self, prompt: &PreviewPrompt) -> BenchmarkResult<recite_core::ChoiceId> {
        if let Some(line) = prompt.line() {
            return self.runtime_fixture.choice_for_line(line.id.as_str());
        }
        prompt
            .choices()
            .first()
            .map(|choice| choice.id.clone())
            .ok_or_else(|| error("preview prompt has no selectable choices"))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewRestoreParity {
    pub events_match: bool,
    pub original_event_count: usize,
    pub restored_event_count: usize,
}

fn initial_output(
    preview: &mut PreviewSession<'_>,
    project: &PreviewProject,
) -> BenchmarkResult<recite_runtime::PreviewOutput> {
    match preview.state().status() {
        PreviewStatus::Ready => Ok(preview.step(project.inputs())),
        PreviewStatus::WaitingForChoice { prompt } => {
            let choice = project.choice_for_prompt(prompt)?;
            Ok(preview.choose(choice, project.inputs()))
        }
        PreviewStatus::WaitingForEffect { effect } => {
            acknowledge_blocking(preview, effect.id.clone())
        }
        PreviewStatus::Ended => Err(error("preview was already ended")),
        PreviewStatus::WaitingForCondition { .. } => Err(error(
            "preview cannot drive a session with an unanswered condition",
        )),
        _ => Err(error(
            "preview status is not supported by the benchmark driver",
        )),
    }
}

fn answer_condition(
    preview: &mut PreviewSession<'_>,
    request_id: recite_runtime::PreviewConditionRequestId,
    answer: recite_runtime::ConditionValue,
    project: &PreviewProject,
) -> BenchmarkResult<recite_runtime::PreviewOutput> {
    Ok(preview.answer(request_id, ConditionAnswer::Value(answer), project.inputs()))
}

fn acknowledge_blocking(
    preview: &mut PreviewSession<'_>,
    effect_id: EffectId,
) -> BenchmarkResult<recite_runtime::PreviewOutput> {
    Ok(preview.acknowledge(effect_id, recite_runtime::EffectAck::Completed))
}

fn preview_error(preview: recite_runtime::PreviewError) -> crate::BenchmarkError {
    crate::error(format!("preview operation failed: {preview}"))
}
