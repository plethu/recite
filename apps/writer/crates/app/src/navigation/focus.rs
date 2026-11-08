//! Native fields retain selection/history; only the admitted passage owns the product draft.
use super::*;
pub use crate::Phase;
pub use crate::attention::{EditingTarget, FieldRestoration};

#[derive(Default)]
pub(super) enum Admission {
    #[default]
    Idle,
    Applying {
        request: Arc<Request>,
        latest: Option<EditingTarget>,
    },
    Retiring {
        request: Arc<Request>,
        latest: Option<EditingTarget>,
        accepted: bool,
    },
    Restoring(FieldRestoration),
}
impl Admission {
    pub fn busy(&self) -> bool {
        matches!(self, Self::Applying { .. } | Self::Retiring { .. })
    }
}
impl Writer {
    pub fn field_restoration(&self) -> Option<FieldRestoration> {
        match &self.navigation.field {
            Admission::Restoring(target) => Some(target.clone()),
            _ => None,
        }
    }
    pub fn native_field_focus(&mut self, target: Option<EditingTarget>) {
        if target.is_none() {
            self.attention.set_field(None);
        }
        match &mut self.navigation.field {
            Admission::Applying { latest, .. } | Admission::Retiring { latest, .. } => {
                *latest = target;
                return;
            }
            Admission::Idle | Admission::Restoring(_) => self.navigation.field = Admission::Idle,
        }
        let Some(target) = target.filter(|target| target.belongs_to(self)) else {
            return;
        };
        if EditingTarget::capture(self).as_ref() == Some(&target) {
            self.attention.set_field(Some(target));
            return;
        }
        if self.phase != Phase::Writing
            || self.attention.is_composing()
            || self.session.read_only()
            || self.navigation.busy()
            || self.work.busy()
        {
            self.restore_field();
            return;
        }
        if !self.session.model().has_draft() {
            self.admit_clean_field(target);
            return;
        }
        if let Err(error) = self.begin_navigation(Target::Edit(EditOperation::Apply)) {
            self.status = error;
            self.restore_field();
        } else if let Some(request) = self.navigation.pending.clone() {
            self.status = "Checking previous passage…".into();
            self.navigation.field = Admission::Applying {
                request,
                latest: Some(target),
            };
        }
    }
    fn admit_clean_field(&mut self, target: EditingTarget) {
        if !target.belongs_to(self) {
            return;
        }
        let result = self.session.edit(|model| model.select(target.view()));
        match result {
            Ok(()) => {
                self.attention.set_field(Some(target));
                self.retire_rules();
                self.reading.prepare(self.session.model());
                self.sync_map_selection();
                self.remember_location();
                self.status = "Ready".into();
            }
            Err(error) => {
                self.status = feedback::file(&error);
                self.restore_field();
            }
        }
    }
    fn restore_field(&mut self) {
        self.navigation.field = Admission::Restoring(
            EditingTarget::capture(self).map_or(FieldRestoration::Clear, FieldRestoration::Passage),
        );
    }
    pub(super) fn field_apply_completed(&mut self, completed: &Arc<Request>, accepted: bool) {
        if matches!(&self.navigation.field, Admission::Applying { request, .. } if Arc::ptr_eq(request, completed))
        {
            let Admission::Applying { request, latest } =
                std::mem::take(&mut self.navigation.field)
            else {
                return;
            };
            self.navigation.field = Admission::Retiring {
                request,
                latest,
                accepted: accepted && !self.session.model().has_draft(),
            };
        }
    }
    pub(super) fn field_apply_retired(&mut self, completed: &Arc<Request>) {
        if !matches!(&self.navigation.field, Admission::Retiring { request, .. } if Arc::ptr_eq(request, completed))
        {
            return;
        }
        let Admission::Retiring {
            latest, accepted, ..
        } = std::mem::take(&mut self.navigation.field)
        else {
            return;
        };
        if let Some(target) = latest {
            if accepted && self.phase == Phase::Writing && !self.session.read_only() {
                self.admit_clean_field(target);
            } else {
                self.restore_field();
            }
        }
    }
}

#[path = "focus/tests.rs"]
#[cfg(test)]
mod tests;
