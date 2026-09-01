use recite_core::{ChoiceId, EffectId};

use super::PreviewSession;
use super::condition::Operation;
use super::model::{
    ConditionAnswer, PreviewCommand, PreviewConditionRequestId, PreviewInputs, PreviewOutput,
};
use crate::EffectAck;

impl<'asset> PreviewSession<'asset> {
    /// Advances traversal by at most one externally visible runtime event.
    pub fn step(&mut self, inputs: PreviewInputs<'_>) -> PreviewOutput {
        let prompt = self.prompt_identity_for_session(&self.session);
        self.run_operation(Operation::Advance { prompt }, inputs)
    }

    /// Selects a stable choice ID from the current prompt.
    pub fn choose(&mut self, choice_id: ChoiceId, inputs: PreviewInputs<'_>) -> PreviewOutput {
        let prompt = self.pending_prompt_identity();
        self.run_operation(Operation::Choose { choice_id, prompt }, inputs)
    }

    /// Supplies the answer for the exact condition request currently pending.
    pub fn answer(
        &mut self,
        request_id: PreviewConditionRequestId,
        answer: ConditionAnswer,
        inputs: PreviewInputs<'_>,
    ) -> PreviewOutput {
        self.dispatch(PreviewCommand::Answer { request_id, answer }, inputs)
    }

    /// Acknowledges the exact pending blocking effect.
    pub fn acknowledge(&mut self, effect_id: EffectId, ack: EffectAck) -> PreviewOutput {
        self.dispatch(
            PreviewCommand::Acknowledge { effect_id, ack },
            PreviewInputs::default(),
        )
    }

    /// Applies one preview command. Recoverable failures are typed error events.
    pub fn dispatch(
        &mut self,
        command: PreviewCommand,
        inputs: PreviewInputs<'_>,
    ) -> PreviewOutput {
        match command {
            PreviewCommand::Advance => self.step(inputs),
            PreviewCommand::Choose { choice_id } => self.choose(choice_id, inputs),
            PreviewCommand::Answer { request_id, answer } => {
                self.answer_pending(request_id, answer, inputs)
            }
            PreviewCommand::Acknowledge { effect_id, ack } => {
                self.acknowledge_pending(effect_id, ack)
            }
            PreviewCommand::Restart => self.restart(),
        }
    }
}
