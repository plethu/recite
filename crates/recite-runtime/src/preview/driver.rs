use super::PreviewSession;
use super::condition::{Operation, PendingOperation, ReplayContext};
use super::model::{
    ConditionAnswer, PreviewConditionRequest, PreviewConditionRequestId, PreviewError,
    PreviewEvent, PreviewInputs, PreviewOutput, PreviewStatus,
};
use super::projection::new_deferred_events;
use super::trial::Trial;
use crate::{DialogueError, LocaleResolution, choose_with, next_with};

impl<'asset> PreviewSession<'asset> {
    pub(super) fn run_operation(
        &mut self,
        operation: Operation,
        inputs: PreviewInputs<'_>,
    ) -> PreviewOutput {
        if self.pending.is_some() {
            return self.error(PreviewError::ConditionPending);
        }
        let base = self.session.clone();
        let prior_status = self.state.status.clone();
        self.run_trial(Trial {
            operation,
            base,
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
        let Some(pending) = self.pending.clone() else {
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

        let mut events = vec![PreviewEvent::ConditionResult {
            request: request.clone(),
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
            base: pending.base,
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
            base,
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

        let (result, pending_query, mismatch, trial) = {
            let context = ReplayContext::new(&answers, &requests);
            let mut trial = base.clone();
            let result = match &operation {
                Operation::Advance { .. } => {
                    next_with(self.asset, &mut trial, &context, resolution)
                }
                Operation::Choose { choice_id, .. } => choose_with(
                    self.asset,
                    &mut trial,
                    choice_id.clone(),
                    &context,
                    resolution,
                ),
            };
            (result, context.pending_query(), context.mismatch(), trial)
        };

        if let Some(mismatch) = mismatch {
            self.pending = None;
            self.state.status = prior_status;
            return self.error_with_prefix(
                prefix,
                PreviewError::ConditionReplayMismatch {
                    mismatch: mismatch.to_string(),
                },
            );
        }
        if let Some(query) = pending_query {
            let Some(block) = self.block_id_for_session(&trial) else {
                self.pending = None;
                self.state.status = prior_status;
                return self.error_with_prefix(
                    prefix,
                    PreviewError::Runtime(DialogueError::MalformedCompiledAsset {
                        reason: "condition query has no active block".to_owned(),
                    }),
                );
            };
            let Some(next_id) = self
                .next_condition_id
                .get()
                .checked_add(1)
                .map(PreviewConditionRequestId::new)
            else {
                self.pending = None;
                self.state.status = prior_status;
                return self.error_with_prefix(prefix, PreviewError::ConditionRequestIdOverflow);
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
                base,
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

        let event = match result {
            Ok(event) => event,
            Err(error) => {
                self.pending = None;
                self.state.status = prior_status;
                return self.error_with_prefix(prefix, PreviewError::Runtime(error));
            }
        };
        let Some(block) = self.block_id_for_session(&trial) else {
            self.pending = None;
            self.state.status = prior_status;
            return self.error_with_prefix(
                prefix,
                PreviewError::Runtime(DialogueError::MalformedCompiledAsset {
                    reason: "runtime event has no active block".to_owned(),
                }),
            );
        };

        self.session = trial;
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
        events.extend(new_deferred_events(&base, &self.session));
        events.push(PreviewEvent::from_dialogue_event(
            event,
            block,
            runtime_trace.plural_arm_count(),
        ));
        self.append_events(events)
    }
}
