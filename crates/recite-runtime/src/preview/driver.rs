use super::PreviewSession;
use super::condition::{Operation, PendingOperation, ReplayContext};
use super::model::{
    ConditionAnswer, PreviewConditionRequest, PreviewConditionRequestId, PreviewError,
    PreviewEvent, PreviewInputs, PreviewOutput, PreviewStatus,
};
use super::projection::new_deferred_events;
use super::trial::{Trial, TrialFailure};
use crate::LocaleResolution;

impl<'asset> PreviewSession<'asset> {
    pub(super) fn run_operation(
        &mut self,
        operation: Operation,
        inputs: PreviewInputs<'_>,
    ) -> PreviewOutput {
        if self.pending.is_some() {
            return self.error(PreviewError::ConditionPending);
        }
        let prior_status = self.state.status.clone();
        self.run_trial(Trial {
            operation,
            answers: Vec::new(),
            requests: Vec::new(),
            inputs,
            prior_status,
            prefix: None,
            runtime_trace: crate::DialogueTrace::new(),
        })
    }

    pub(super) fn answer_pending(
        &mut self,
        request_id: PreviewConditionRequestId,
        answer: ConditionAnswer,
        inputs: PreviewInputs<'_>,
    ) -> PreviewOutput {
        let Some(pending) = self.pending.as_ref() else {
            return self.error(PreviewError::ConditionNotPending);
        };
        let Some(request) = pending.requests.last() else {
            return self.error(PreviewError::ConditionNotPending);
        };
        if pending.input_revision != inputs.revision {
            return self.error(PreviewError::InputRevisionMismatch {
                expected: pending.input_revision,
                actual: inputs.revision,
            });
        }
        if request.id != request_id {
            return self.error(PreviewError::ConditionRequestMismatch {
                expected: request.id,
                actual: request_id,
            });
        }
        if let ConditionAnswer::Value(value) = &answer
            && value.kind() != request.query.expected_type()
        {
            return self.error(PreviewError::ConditionResultTypeMismatch {
                function: request.query.function().to_owned(),
                expected: request.query.expected_type(),
                actual: value.kind(),
            });
        }

        let request = request.clone();
        // Rejected answers leave the pending operation available for retry.
        // Once validated, replay owns it instead of cloning its requests.
        let Some(pending) = self.pending.take() else {
            return self.error(PreviewError::ConditionNotPending);
        };
        let mut events = vec![PreviewEvent::ConditionResult {
            request,
            result: super::model::PreviewConditionResult::from_answer(&answer),
        }];
        if let ConditionAnswer::Failed { reason } = answer {
            self.pending = None;
            self.state.status = pending.prior_status;
            events.push(PreviewEvent::Error(PreviewError::ConditionFailed {
                request_id,
                reason,
            }));
            return self.append_events(events);
        }

        let mut answers = pending.answers;
        answers.push(answer);
        self.run_trial(Trial {
            operation: pending.operation,
            answers,
            requests: pending.requests,
            inputs,
            prior_status: pending.prior_status,
            prefix: Some(events),
            runtime_trace: crate::DialogueTrace::new(),
        })
    }

    fn run_trial(&mut self, trial: Trial<'_>) -> PreviewOutput {
        let Trial {
            operation,
            answers,
            mut requests,
            inputs,
            prior_status,
            prefix,
            runtime_trace,
        } = trial;
        let mut resolution = LocaleResolution::new()
            .with_trace(&runtime_trace)
            .with_preview_plural_arm_validation();
        if let Some(provider) = inputs.locale_provider {
            resolution = resolution.with_provider(provider);
        }
        if let Some(values) = inputs.interpolation_values {
            resolution = resolution.with_values(values);
        }
        if let Some(variant) = self.options.variant.as_deref() {
            resolution = resolution.with_variant(variant);
        }

        let previous_deferred = self.session.deferred_effects().len();
        let context = ReplayContext::new(&answers, &requests);
        let result = super::trial::traverse(
            &mut self.session,
            &mut self.state.deferred_effects,
            self.asset,
            &operation,
            &context,
            resolution,
        );
        let (event, block) = match result {
            Ok(committed) => committed,
            Err(TrialFailure::Error(error)) => {
                self.pending = None;
                self.state.status = prior_status;
                return self.error_with_prefix(prefix, error);
            }
            Err(TrialFailure::Pending { query, block }) => {
                let Some(next_id) = self
                    .next_condition_id
                    .get()
                    .checked_add(1)
                    .map(PreviewConditionRequestId::new)
                else {
                    self.pending = None;
                    self.state.status = prior_status;
                    return self
                        .error_with_prefix(prefix, PreviewError::ConditionRequestIdOverflow);
                };
                let request = PreviewConditionRequest {
                    id: self.next_condition_id,
                    block,
                    prompt: operation.prompt().cloned(),
                    query,
                };
                self.next_condition_id = next_id;
                requests.push(request.clone());
                let accepted_choice = if requests.len() == 1 {
                    match &operation {
                        Operation::Choose {
                            prompt: Some(prompt),
                            choice_id,
                        } => Some((prompt.clone(), choice_id.clone())),
                        _ => None,
                    }
                } else {
                    None
                };
                self.pending = Some(PendingOperation {
                    operation,
                    answers,
                    requests,
                    prior_status,
                    input_revision: inputs.revision,
                });
                self.state.status = PreviewStatus::WaitingForCondition {
                    request: request.clone(),
                };
                let mut events = prefix.unwrap_or_default();
                if let Some((prompt, choice_id)) = accepted_choice {
                    events.push(PreviewEvent::ChoiceAccepted { prompt, choice_id });
                }
                events.push(PreviewEvent::ConditionRequested(request));
                return self.append_events(events);
            }
        };

        self.pending = None;
        self.trace.merge_runtime_trace(&runtime_trace);
        if let crate::DialogueEvent::Effect(effect) = &event
            && self.restored_effect_reemit.as_ref() == Some(&effect.id)
        {
            self.restored_effect_reemit = None;
        }
        let mut events = prefix.unwrap_or_default();
        if let Operation::Choose {
            prompt: Some(prompt),
            choice_id,
        } = &operation
        {
            events.push(PreviewEvent::ChoiceSelected {
                prompt: prompt.clone(),
                choice_id: choice_id.clone(),
            });
        }
        events.extend(new_deferred_events(previous_deferred, &self.session));
        events.push(PreviewEvent::from_dialogue_event(
            event,
            block,
            runtime_trace.plural_arm_count(),
        ));
        self.append_events(events)
    }
}
