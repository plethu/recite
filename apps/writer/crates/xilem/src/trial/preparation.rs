//! Terminal feedback is distinct from the worker slot and the retained conversation.
use super::{Request, Review, Trial};
use std::sync::Arc;
pub(super) const GUIDANCE: &str =
    "Effects are displayed, never executed. Restart captures your staged inputs.";

pub(super) enum Preparation {
    Idle(Outcome),
    Running(Arc<Request>),
    Cancelling,
}

pub(super) enum Outcome {
    Ready,
    Failed(String),
    Cancelled,
    Changed,
}

impl Trial {
    pub(crate) fn document_replaced(&mut self) {
        self.settings.review = Review::Inputs;
        if !self.busy() {
            self.preparation = Preparation::Idle(Outcome::Ready);
            self.status = GUIDANCE.into();
        }
    }
    pub fn busy(&self) -> bool {
        matches!(
            self.preparation,
            Preparation::Running(_) | Preparation::Cancelling
        )
    }

    pub fn failed(&self) -> bool {
        self.failure().is_some()
    }

    pub fn failure(&self) -> Option<&str> {
        match &self.preparation {
            Preparation::Idle(Outcome::Failed(error)) => Some(error),
            _ => None,
        }
    }

    pub fn feedback(&self, retained: bool) -> Option<&'static str> {
        match self.preparation {
            Preparation::Idle(Outcome::Failed(_)) if retained => {
                Some("Preview failed. Existing conversation kept.")
            }
            Preparation::Idle(Outcome::Failed(_)) => Some("Preview could not start."),
            Preparation::Idle(Outcome::Cancelled) => Some("Preparation cancelled."),
            Preparation::Idle(Outcome::Changed) => {
                Some("Earlier preparation discarded. Restart when ready.")
            }
            _ => None,
        }
    }

    pub fn submission_caption(&self, retained: bool) -> &'static str {
        match self.preparation {
            Preparation::Idle(_) if retained => "Restart preview",
            Preparation::Idle(_) => "Start preview",
            Preparation::Running(_) => "Preparing preview…",
            Preparation::Cancelling => "Cancelling preparation…",
        }
    }

    pub fn can_cancel(&self) -> bool {
        matches!(self.preparation, Preparation::Running(_))
    }

    pub fn cancel(&mut self) {
        if self.busy() {
            self.preparation = Preparation::Cancelling;
            self.status = "Cancelled; finishing the current compiler phase.".into();
        }
    }
}
