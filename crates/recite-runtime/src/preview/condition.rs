use std::cell::RefCell;

use crate::{ConditionEvaluationError, ConditionValue, DialogueContext, PreviewConditionQuery};

use super::model::{
    ConditionAnswer, PreviewConditionRequest, PreviewInputRevision, PreviewPromptIdentity,
};

/// One externally visible operation that can be replayed after an answer.
#[derive(Clone, Debug)]
pub(super) enum Operation {
    Advance {
        prompt: Option<PreviewPromptIdentity>,
    },
    Choose {
        choice_id: recite_core::ChoiceId,
        prompt: Option<PreviewPromptIdentity>,
    },
}

impl Operation {
    pub(super) fn prompt(&self) -> Option<&PreviewPromptIdentity> {
        match self {
            Self::Advance { prompt } => prompt.as_ref(),
            Self::Choose { prompt, .. } => prompt.as_ref(),
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct PendingOperation {
    pub(super) operation: Operation,
    pub(super) base: crate::DialogueSession,
    pub(super) answers: Vec<ConditionAnswer>,
    pub(super) requests: Vec<PreviewConditionRequest>,
    pub(super) prior_status: super::model::PreviewStatus,
    pub(super) input_revision: PreviewInputRevision,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ReplayMismatch {
    pub(super) index: usize,
    pub(super) expected: PreviewConditionQuery,
    pub(super) actual: PreviewConditionQuery,
}

impl std::fmt::Display for ReplayMismatch {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "query {} changed from `{}` to `{}`",
            self.index,
            self.expected.function(),
            self.actual.function()
        )
    }
}

/// Context used for transactional traversal. It replays already supplied
/// answers and stops at the first query without an answer; no trial session is
/// committed until traversal returns an externally visible event.
pub(super) struct ReplayContext<'a> {
    state: RefCell<ReplayState<'a>>,
}

struct ReplayState<'a> {
    answers: &'a [ConditionAnswer],
    requests: &'a [PreviewConditionRequest],
    cursor: usize,
    pending: Option<PreviewConditionQuery>,
    mismatch: Option<ReplayMismatch>,
}

impl<'a> ReplayContext<'a> {
    pub(super) fn new(
        answers: &'a [ConditionAnswer],
        requests: &'a [PreviewConditionRequest],
    ) -> Self {
        Self {
            state: RefCell::new(ReplayState {
                answers,
                requests,
                cursor: 0,
                pending: None,
                mismatch: None,
            }),
        }
    }

    pub(super) fn pending_query(&self) -> Option<PreviewConditionQuery> {
        self.state.borrow().pending.clone()
    }

    pub(super) fn mismatch(&self) -> Option<ReplayMismatch> {
        self.state.borrow().mismatch.clone()
    }
}

impl ReplayState<'_> {
    fn evaluate(
        &mut self,
        actual: PreviewConditionQuery,
    ) -> Result<ConditionValue, ConditionEvaluationError> {
        let index = self.cursor;
        if let Some(request) = self.requests.get(index) {
            if request.query() != &actual {
                self.mismatch = Some(ReplayMismatch {
                    index,
                    expected: request.query().clone(),
                    actual,
                });
                return Err(ConditionEvaluationError::new(
                    "condition replay query no longer matches",
                ));
            }
        } else {
            self.pending = Some(actual);
            return Err(ConditionEvaluationError::new(
                "preview condition answer is pending",
            ));
        }

        let Some(answer) = self.answers.get(index) else {
            self.pending = self
                .requests
                .get(index)
                .map(|request| request.query().clone());
            return Err(ConditionEvaluationError::new(
                "preview condition answer is pending",
            ));
        };
        self.cursor = self.cursor.saturating_add(1);
        match answer {
            ConditionAnswer::Value(value) => Ok(value.clone()),
            ConditionAnswer::Failed { reason } => {
                Err(ConditionEvaluationError::new(reason.clone()))
            }
        }
    }
}

impl DialogueContext for ReplayContext<'_> {
    fn evaluate_condition(
        &self,
        query: crate::ConditionQuery<'_>,
    ) -> Result<ConditionValue, ConditionEvaluationError> {
        self.state
            .borrow_mut()
            .evaluate(PreviewConditionQuery::from_query(query))
    }
}
