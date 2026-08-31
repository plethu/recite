use super::condition::Operation;
use super::model::{
    ConditionAnswer, PreviewConditionRequest, PreviewEvent, PreviewInputs, PreviewStatus,
};

pub(super) struct Trial<'a> {
    pub(super) operation: Operation,
    pub(super) base: crate::DialogueSession,
    pub(super) answers: Vec<ConditionAnswer>,
    pub(super) requests: Vec<PreviewConditionRequest>,
    pub(super) inputs: PreviewInputs<'a>,
    pub(super) prior_status: PreviewStatus,
    pub(super) prefix: Option<Vec<PreviewEvent>>,
    pub(super) runtime_trace: crate::DialogueTrace,
}
