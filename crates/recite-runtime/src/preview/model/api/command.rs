use recite_core::{ChoiceId, EffectId};

use crate::EffectAck;

use super::PreviewConditionRequestId;

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum PreviewCommand {
    Advance,
    Choose {
        choice_id: ChoiceId,
    },
    Answer {
        request_id: PreviewConditionRequestId,
        answer: ConditionAnswer,
    },
    Acknowledge {
        effect_id: EffectId,
        ack: EffectAck,
    },
    Restart,
}

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum ConditionAnswer {
    Value(crate::ConditionValue),
    Failed { reason: String },
}
