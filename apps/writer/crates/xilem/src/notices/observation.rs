//! Only matching saved baselines may publish advisory evidence.
use super::*;

impl Writer {
    pub(super) fn accept_external_notice(&mut self, output: Output) {
        if output.window != self.window {
            return;
        }
        match output.event {
            Event::Sources(changes) => {
                if let Some(scope) = self.localisation.watch_scope()
                    && changes.affected.contains(scope.path())
                {
                    self.notices.pending.catalogue = true;
                }
                self.notices.pending.sources.merge(changes.affected);
                if changes.error.is_some() {
                    self.notices.warning = changes.error;
                }
            }
            Event::Catalogue(scope, changes)
                if self.localisation.watch_scope().as_ref() == Some(&scope) =>
            {
                if changes.affected.contains(scope.path()) {
                    self.notices.pending.catalogue = true;
                }
                if changes.error.is_some() {
                    self.notices.warning = changes.error;
                }
            }
            Event::Bound(scope, warning)
                if self.localisation.watch_scope().as_ref() == Some(&scope) =>
            {
                self.notices.pending.catalogue = true;
                if warning.is_some() {
                    self.notices.warning = warning;
                }
            }
            Event::Observed(observed) => self.publish_external_observation(observed),
            Event::Warning(warning) => self.notices.warning = Some(warning),
            Event::Stopped(warning) => {
                self.notices.warning = Some(warning);
                self.notices.connection = None;
                self.notices.flight = None;
            }
            _ => (),
        }
    }
    fn publish_external_observation(&mut self, observed: Observed) {
        if !self
            .notices
            .flight
            .as_ref()
            .is_some_and(|flight| flight.matches(&observed.flight))
        {
            return;
        }
        self.notices.flight = None;
        for observation in observed.sources {
            let target = Target::Source(observation.path().to_owned());
            let current = !self.session.read_only()
                && self.session.project().is_some_and(|project| {
                    !project.files().saving && observation.is_current(project.files())
                });
            if !current {
                self.notices.found.remove(&target);
                self.notices
                    .pending
                    .sources
                    .insert(observation.path().to_owned());
            } else if observation.changed() {
                self.notices
                    .found
                    .insert(target, Evidence::Source(observation));
            } else {
                self.notices.found.remove(&target);
            }
        }
        if let Some(observation) = observed.catalogue
            && self.localisation.watch_scope().as_ref() == Some(observation.scope())
        {
            let target = Target::Catalogue(observation.scope().path().to_owned());
            if !observation.is_current(&self.localisation) {
                self.notices.found.remove(&target);
                self.notices.pending.catalogue = true;
            } else if observation.changed() {
                self.notices
                    .found
                    .insert(target, Evidence::Catalogue(observation));
            } else {
                self.notices.found.remove(&target);
            }
        }
    }
}
