//! The existing project-tools worker carries catalogue owners and save receipts.
mod checked;
mod saving;
mod source;
use super::*;
pub(super) use checked::Action;
use recite_writer_model::Passage;
pub(super) use saving::AfterSave;
pub(crate) use saving::{Finish, Finished};
use slot::Checkout;
pub(super) use source::Action as SourceAction;
use std::sync::Arc;

pub(crate) struct Work {
    kind: Kind,
}
enum Kind {
    Checked(checked::Checked),
    Saving(saving::Save),
    Source(source::Work),
}
pub(crate) struct Completed {
    kind: Completion,
}
enum Completion {
    Checked(Box<checked::CheckedCompleted>),
    Saving(saving::Saved),
    Source(Box<source::Completed>),
}
impl Work {
    pub(super) fn checked(
        checkout: Checkout,
        action: Action,
        query: Query,
        passages: Option<Arc<[Passage]>>,
    ) -> Self {
        Self {
            kind: Kind::Checked(checked::Checked::new(checkout, action, query, passages)),
        }
    }
    pub(crate) fn not_started(self, error: String) -> Completed {
        Completed {
            kind: match self.kind {
                Kind::Checked(work) => Completion::Checked(Box::new(work.not_started(error))),
                Kind::Source(work) => Completion::Source(Box::new(work.not_started(error))),
                Kind::Saving(work) => Completion::Saving(work.not_started(error)),
            },
        }
    }
    pub(crate) fn execute(self) -> Completed {
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
    pub(crate) fn accept_catalogue_tool(
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
