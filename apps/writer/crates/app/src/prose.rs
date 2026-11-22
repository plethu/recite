//! Prose findings for the open document in projects with `[prose] enabled`.
//!
//! After each native message the Writer compares the accepted source with the one last sent for
//! checking and, when it changed, asks the frontend's worker to check it off the UI thread. Each
//! request carries a generation; only the newest result is kept, and only while the document
//! still holds the source it was checked against. A project without `[prose]` captures no
//! configuration, so none of this allocates, sends or builds a checker.
mod fixes;
mod problems;
use crate::Writer;
pub use problems::{Problem, Problems, ProseProblem};
pub use recite_writer_session::prose::{Checker, Configuration, Outcome};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

/// Which prose check a result, problem or fix belongs to. Each request is newer than the last.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CheckGeneration(u64);

impl CheckGeneration {
    const fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

#[derive(Default)]
pub struct Prose {
    pub sender: Option<UnboundedSender<Request>>,
    generation: CheckGeneration,
    requested: Option<Stamp>,
    checked: Option<Checked>,
}

/// What the latest request covered.
struct Stamp {
    configuration: Arc<Configuration>,
    source: Arc<str>,
}

/// One document source to check.
pub struct Request {
    generation: CheckGeneration,
    configuration: Arc<Configuration>,
    key: String,
    source: Arc<str>,
}

/// A checked source and its outcome.
pub struct Checked {
    generation: CheckGeneration,
    configuration: Arc<Configuration>,
    source: Arc<str>,
    outcome: Outcome,
}

impl std::fmt::Debug for Checked {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProseChecked")
            .field("generation", &self.generation)
            .field("problems", &self.outcome.len())
            .finish_non_exhaustive()
    }
}

impl Request {
    /// Run on a worker; `checker` keeps per-project checkers and per-document findings.
    pub fn execute(self, checker: &mut Checker) -> Checked {
        let outcome = checker.check(&self.configuration, &self.key, &self.source);
        Checked {
            generation: self.generation,
            configuration: self.configuration,
            source: self.source,
            outcome,
        }
    }
}

impl Writer {
    /// Ask for the accepted source to be checked when it, or the project's prose settings,
    /// changed since the last request. Frontends call this after every native message.
    pub fn check_prose_if_needed(&mut self) {
        if self.session.checked_out() {
            return;
        }
        let Some(configuration) = self
            .session
            .project()
            .and_then(|project| project.files().prose())
        else {
            if self.prose.requested.is_some() || self.prose.checked.is_some() {
                self.prose.requested = None;
                self.prose.checked = None;
            }
            return;
        };
        let document = self.session.model().document();
        if let Some(stamp) = &mut self.prose.requested
            && (Arc::ptr_eq(&stamp.configuration, configuration)
                || stamp.configuration == *configuration)
        {
            if !Arc::ptr_eq(&stamp.configuration, configuration) {
                stamp.configuration = configuration.clone();
            }
            if std::ptr::eq(stamp.source.as_ref(), document.source()) {
                return;
            }
            // The same text in a new allocation, as after a save, keeps its findings.
            if stamp.source.as_ref() == document.source() {
                stamp.source = document.source_snapshot();
                if let Some(checked) = &mut self.prose.checked
                    && checked.source.as_ref() == document.source()
                {
                    checked.source = stamp.source.clone();
                }
                return;
            }
        }
        let Some(sender) = self
            .prose
            .sender
            .as_ref()
            .filter(|sender| !sender.is_closed())
        else {
            return;
        };
        let generation = self.prose.generation.next();
        let request = Request {
            generation,
            configuration: configuration.clone(),
            key: document.key().as_str().to_owned(),
            source: document.source_snapshot(),
        };
        let stamp = Stamp {
            configuration: request.configuration.clone(),
            source: request.source.clone(),
        };
        if sender.send(request).is_ok() {
            self.prose.generation = generation;
            self.prose.requested = Some(stamp);
        }
    }

    /// Keep a worker's result when it answers the newest request.
    pub fn accept_prose(&mut self, mut checked: Checked) {
        let Some(stamp) = &self.prose.requested else {
            return;
        };
        if checked.generation == self.prose.generation {
            // The request's text may have moved to a new allocation while it was checked.
            if !Arc::ptr_eq(&checked.source, &stamp.source) && checked.source == stamp.source {
                checked.source = stamp.source.clone();
            }
            self.prose.checked = Some(checked);
        }
    }

    /// The newest result, while the document still holds the source it checked.
    fn current_prose(&self) -> Option<&Checked> {
        self.prose.checked.as_ref().filter(|checked| {
            checked.generation == self.prose.generation
                && std::ptr::eq(
                    checked.source.as_ref(),
                    self.session.model().document().source(),
                )
        })
    }

    /// Check again even though the source is unchanged, as after a dictionary edit.
    fn recheck_prose(&mut self) {
        self.prose.requested = None;
        self.check_prose_if_needed();
    }
}

#[cfg(test)]
mod tests;
