//! Advisory disk changes never apply drafts or adopt a file without explicit comparison.
pub mod listener;
pub mod observation;
pub mod worker;
use crate::{
    Phase, Writer,
    localisation::{WatchObservation, WatchRequest, WatchScope},
};
use recite_writer_session::{
    external::{Affected, Changes},
    project::{ExternalCheck, ExternalObservation},
};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
use tokio::sync::{mpsc, watch};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Target {
    Source(PathBuf),
    Catalogue(PathBuf),
}
pub enum Evidence {
    Source(ExternalObservation),
    Catalogue(WatchObservation),
}
#[derive(Clone)]
pub struct Flight(Arc<()>);
impl Flight {
    fn matches(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
pub struct Connection {
    requests: mpsc::Sender<Request>,
    catalogue: watch::Sender<Option<WatchScope>>,
}
#[derive(Default)]
pub struct Pending {
    sources: Affected,
    catalogue: bool,
}
#[derive(Default)]
pub struct Notices {
    connection: Option<Connection>,
    flight: Option<Flight>,
    pending: Pending,
    pub found: BTreeMap<Target, Evidence>,
    cursor: Option<Target>,
    pub warning: Option<String>,
}
pub struct Request {
    flight: Flight,
    sources: Vec<ExternalCheck>,
    catalogue: Option<WatchRequest>,
}
pub struct Observed {
    flight: Flight,
    sources: Vec<ExternalObservation>,
    catalogue: Option<WatchObservation>,
}
pub enum Event {
    Sources(Changes),
    Catalogue(WatchScope, Changes),
    Bound(WatchScope, Option<String>),
    Observed(Observed),
    Warning(String),
    Stopped(String),
}
pub struct Output {
    pub window: crate::WindowKey,
    pub event: Event,
}
impl std::fmt::Debug for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiskObservation").finish_non_exhaustive()
    }
}
impl Notices {
    pub fn selected(&self) -> Option<(&Target, &Evidence)> {
        self.cursor
            .as_ref()
            .and_then(|cursor| self.found.get_key_value(cursor))
            .or_else(|| self.found.first_key_value())
    }
    pub fn next(&mut self) {
        use std::ops::Bound::{Excluded, Unbounded};
        self.cursor = self
            .selected()
            .and_then(|(key, _)| self.found.range((Excluded(key.clone()), Unbounded)).next())
            .or_else(|| self.found.first_key_value())
            .map(|(key, _)| key.clone());
    }
}
impl Request {
    pub fn observe(self) -> Observed {
        Observed {
            flight: self.flight,
            sources: self
                .sources
                .into_iter()
                .map(ExternalCheck::observe)
                .collect(),
            catalogue: self.catalogue.map(WatchRequest::observe),
        }
    }
}
impl Writer {
    pub fn resume_external_notices(&mut self) {
        let scope = self.localisation.watch_scope();
        if let Some(connection) = &self.notices.connection
            && *connection.catalogue.borrow() != scope
        {
            self.notices
                .found
                .retain(|target, _| matches!(target, Target::Source(_)));
            self.notices.pending.catalogue = scope.is_some();
            connection.catalogue.send_replace(scope);
        }
        if self.phase != Phase::Writing
            || self.notices.flight.is_some()
            || self.notices.connection.is_none()
        {
            return;
        }
        self.recheck_changed_notice_baseline();
        let sources_ready = self
            .session
            .project()
            .is_some_and(|project| !project.files().saving)
            && !self.session.read_only();
        let sources = if sources_ready {
            self.session
                .project()
                .map(|project| {
                    project
                        .files()
                        .external_checks(&self.notices.pending.sources)
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let catalogue = self
            .notices
            .pending
            .catalogue
            .then(|| self.localisation.watch_request())
            .flatten();
        let catalogue_ready = catalogue.is_some();
        if sources.is_empty() && !catalogue_ready {
            if sources_ready {
                self.notices.pending.sources = Affected::default();
            }
            return;
        }
        let flight = Flight(Arc::new(()));
        let request = Request {
            flight: flight.clone(),
            sources,
            catalogue,
        };
        let Some(connection) = &self.notices.connection else {
            return;
        };
        match connection.requests.try_send(request) {
            Ok(()) => {
                self.notices.flight = Some(flight);
                if sources_ready {
                    self.notices.pending.sources = Affected::default();
                }
                if catalogue_ready {
                    self.notices.pending.catalogue = false;
                }
            }
            Err(error) => {
                self.notices.warning = Some(format!(
                    "Automatic disk-change checks stopped: {error}. Save still checks for conflicts."
                ));
                self.notices.connection = None;
            }
        }
    }
    fn recheck_changed_notice_baseline(&mut self) {
        let Some((target, evidence)) = self.notices.selected() else {
            return;
        };
        let stale = match evidence {
            Evidence::Source(observation) => {
                !self.session.read_only()
                    && self.session.project().is_some_and(|project| {
                        !project.files().saving && !observation.is_current(project.files())
                    })
            }
            Evidence::Catalogue(observation) => {
                self.localisation.watch_request().is_some()
                    && !observation.is_current(&self.localisation)
            }
        };
        if !stale {
            return;
        }
        let target = target.clone();
        self.notices.found.remove(&target);
        match target {
            Target::Source(path) => self.notices.pending.sources.insert(path),
            Target::Catalogue(_) => self.notices.pending.catalogue = true,
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
pub mod tests;
impl Writer {
    pub fn compare_noticed_change(&mut self) {
        if self.phase != Phase::Writing || self.project_action_blocked() {
            return;
        }
        match self.notices.selected() {
            Some((Target::Source(path), Evidence::Source(observation)))
                if self
                    .session
                    .project()
                    .is_some_and(|project| observation.is_current(project.files())) =>
            {
                let path = path.clone();
                if self
                    .session
                    .project()
                    .is_some_and(|project| project.current().path() == path)
                {
                    self.compare_disk_changes();
                } else if let Err(error) =
                    self.begin_navigation(crate::navigation::Target::CompareFile(path))
                {
                    self.status = error;
                }
            }
            Some((_, Evidence::Catalogue(observation)))
                if observation.is_current(&self.localisation) =>
            {
                let scope = observation.scope().clone();
                self.compare_noticed_catalogue(&scope);
            }
            _ => (),
        }
    }
}
