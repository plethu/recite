//! File observation is scoped to the retained catalogue lifetime, including exclusive checkout.
use super::{Identity, Localisation, Writer};
use recite_writer_session::localisation::watching::worker;
use std::path::{Path, PathBuf};
#[cfg(test)]
mod tests;

#[derive(Clone, PartialEq)]
pub(crate) struct Scope {
    identity: Identity,
    path: PathBuf,
}
impl Scope {
    pub fn path(&self) -> &Path {
        &self.path
    }
}
pub(crate) struct Request {
    scope: Scope,
    request: worker::Request,
}
pub(crate) struct Observation {
    scope: Scope,
    output: worker::Output,
}
impl Request {
    pub fn observe(self) -> Observation {
        Observation {
            scope: self.scope,
            output: self.request.read(),
        }
    }
}
impl Observation {
    pub fn scope(&self) -> &Scope {
        &self.scope
    }
    pub fn changed(&self) -> bool {
        self.output.fingerprint.as_ref() != Ok(&self.output.request.baseline)
    }
    pub fn is_current(&self, localisation: &Localisation) -> bool {
        localisation.watch_scope().as_ref() == Some(&self.scope)
            && !localisation.slot.read_only()
            && !localisation.saving()
            && localisation.slot.owner().is_some_and(|owner| {
                owner.catalogue.saved_fingerprint() == &self.output.request.baseline
            })
    }
}
impl Localisation {
    pub fn watch_scope(&self) -> Option<Scope> {
        Some(Scope {
            identity: self.slot.identity()?.clone(),
            path: self.slot.path()?.to_owned(),
        })
    }
    pub fn watch_request(&self) -> Option<Request> {
        if self.slot.read_only() || self.saving() {
            return None;
        }
        let owner = self.slot.owner()?;
        Some(Request {
            scope: self.watch_scope()?,
            request: worker::Request {
                path: owner.catalogue.path.clone(),
                baseline: owner.catalogue.saved_fingerprint().clone(),
            },
        })
    }
}
impl Writer {
    pub(crate) fn compare_noticed_catalogue(&mut self, scope: &Scope) {
        if self.localisation.watch_scope().as_ref() == Some(scope) && self.enter_localisation() {
            self.compare_catalogue();
        }
    }
}
