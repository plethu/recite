//! The existing project-tools worker carries catalogue owners and save receipts.
pub mod checked;
pub mod saving;
pub mod source;
use super::*;
pub use checked::Action;
use recite_writer_model::Passage;
pub use saving::AfterSave;
pub use saving::{Finish, Finished};
use slot::Checkout;
pub use source::Action as SourceAction;
use std::sync::Arc;

pub struct Work {
    kind: Kind,
}
pub enum Kind {
    Checked(checked::Checked),
    Saving(saving::Save),
    Source(source::Work),
}
pub struct Completed {
    kind: Completion,
}
pub enum Completion {
    Checked(Box<checked::CheckedCompleted>),
    Saving(saving::Saved),
    Source(Box<source::Completed>),
}
impl Work {
    pub fn checked(
        checkout: Checkout,
        action: Action,
        query: Query,
        passages: Option<Arc<[Passage]>>,
    ) -> Self {
        Self {
            kind: Kind::Checked(checked::Checked::new(checkout, action, query, passages)),
        }
    }
    pub fn not_started(self, error: String) -> Completed {
        Completed {
            kind: match self.kind {
                Kind::Checked(work) => Completion::Checked(Box::new(work.not_started(error))),
                Kind::Source(work) => Completion::Source(Box::new(work.not_started(error))),
                Kind::Saving(work) => Completion::Saving(work.not_started(error)),
            },
        }
    }
    pub fn execute(self) -> Completed {
        Completed {
            kind: match self.kind {
                Kind::Checked(work) => Completion::Checked(Box::new(work.execute())),
                Kind::Source(work) => Completion::Source(Box::new(work.execute())),
                Kind::Saving(work) => Completion::Saving(work.execute()),
            },
        }
    }
}
impl Writer {
    pub fn accept_catalogue_tool(
        &mut self,
        completed: &mut Completed,
    ) -> Result<Option<Finish>, String> {
        match &mut completed.kind {
            Completion::Checked(completed) => {
                self.accept_checked_catalogue(completed).map(|()| None)
            }
            Completion::Saving(completed) => self.accept_catalogue_save(completed),
            Completion::Source(completed) => self.accept_catalogue_source(completed).map(|()| None),
        }
    }
}
