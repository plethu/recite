//! Terminal feedback is distinct from the worker slot and the retained conversation.
use super::{Request, Review, Trial};
use crate::attention::EditingTarget;
use std::sync::Arc;
pub const GUIDANCE: &str =
    "Effects are displayed, never executed. Restart captures your staged inputs.";

pub enum Preparation {
    Idle(Outcome),
    Running(Arc<Request>),
    Cancelling,
}

pub enum Outcome {
    Ready,
    Failed(String),
    UnfinishedEdit(EditingTarget),
    Cancelled,
    Changed,
}

impl Trial {
    pub fn document_replaced(&mut self) {
        self.context_details = false;
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
            Preparation::Idle(Outcome::UnfinishedEdit(_)) if retained => {
                Some("Finish your manuscript edit before restarting. Your conversation is kept.")
            }
            Preparation::Idle(Outcome::UnfinishedEdit(_)) => {
                Some("Finish your manuscript edit before starting this preview.")
            }
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
