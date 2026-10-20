//! Native history advances after checked publication, with the previous cursor retained on failure.
use super::*;
impl Writer {
    pub(crate) fn history_step(&mut self, forward: bool) {
        if self.work.busy() || self.navigation.busy() || self.phase != crate::Phase::Writing {
            return;
        }
        let Some(step) = self.navigation.history.step(forward) else {
            return;
        };
        let location = step.location().clone();
        self.navigation.visit = Some(step);
        if let Err(error) = self.begin_navigation(Target::Location(location)) {
            self.navigation.visit = None;
            self.status = error;
        }
    }
    pub(crate) fn remember_location(&mut self) {
        let actual = self.current_location();
        if !self
            .navigation
            .visit
            .take()
            .is_some_and(|step| self.navigation.history.accept(step, actual.clone()))
        {
            self.navigation.history.record(actual);
        }
    }
}
