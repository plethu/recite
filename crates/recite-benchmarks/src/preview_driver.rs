use recite_runtime::{ConditionAnswer, PreviewEvent, PreviewPrompt, PreviewSession, PreviewStatus};

use crate::preview::{PreviewProject, PreviewTraversalShape};
use crate::{BenchmarkResult, error};

impl PreviewProject {
    /// Prepares a preview at its first prompt without selecting a choice.
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

    /// Advances through the first choice and its follow-up work, stopping at
    /// the next prompt or at the end boundary for snapshot evidence.
    pub fn after_first_choice(&self) -> BenchmarkResult<PreviewSession<'_>> {
        let mut preview = self.at_first_prompt()?;
        let prompt = match preview.state().status() {
            PreviewStatus::WaitingForChoice { prompt } => prompt.clone(),
            _ => return Err(error("preview did not reach its first choice")),
        };
        let choice_id = self.choice_for_prompt(&prompt)?;
        let mut output = preview.choose(choice_id, self.inputs());
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
                    PreviewEvent::Prompt(_) | PreviewEvent::End { .. } => return Ok(preview),
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

    /// Drives a preview from its current boundary to `End`, retaining events
    /// only for parity assertions and other explicitly inspectable tests.
    pub fn collect_to_end(
        &self,
        preview: &mut PreviewSession<'_>,
    ) -> BenchmarkResult<Vec<PreviewEvent>> {
        let mut events = Vec::new();
        self.drive_to_end(preview, |event| events.push(event.clone()))?;
        Ok(events)
    }

    pub fn traversal_summary(
        &self,
        preview: &mut PreviewSession<'_>,
    ) -> BenchmarkResult<PreviewTraversalShape> {
        let mut event_count = 0;
        let mut event_hash = blake3::Hasher::new();
        self.drive_to_end(preview, |event| {
            event_count += 1;
            crate::preview_hash::hash_event(event, &mut event_hash);
        })?;
        Ok(PreviewTraversalShape {
            event_count,
            event_hash: event_hash.finalize().to_hex().to_string(),
        })
    }

    pub fn full_traversal(&self) -> BenchmarkResult<PreviewTraversalShape> {
        let mut preview = self.start()?;
        self.traversal_summary(&mut preview)
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

    fn drive_to_end(
        &self,
        preview: &mut PreviewSession<'_>,
        mut visit: impl FnMut(&PreviewEvent),
    ) -> BenchmarkResult<()> {
        let mut output = initial_output(preview, self)?;
        loop {
            let mut progressed = false;
            for event in output.events() {
                visit(event);
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
                    PreviewEvent::End { .. } => return Ok(()),
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
    effect_id: recite_core::EffectId,
) -> BenchmarkResult<recite_runtime::PreviewOutput> {
    Ok(preview.acknowledge(effect_id, recite_runtime::EffectAck::Completed))
}
