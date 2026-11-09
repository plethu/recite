//! A refused reading transition belongs to the exact passage/draft that requested it.
use super::*;

pub struct PassageRefusal {
    request: Arc<Request>,
    reason: String,
    expanded: bool,
}
impl PassageRefusal {
    pub fn title(&self) -> &'static str {
        if matches!(self.request.target, Target::PreviewEdit(_)) {
            "Can't preview this edit yet"
        } else {
            "Can't continue yet"
        }
    }
    pub fn footer(&self) -> &'static str {
        if matches!(self.request.target, Target::PreviewEdit(_)) {
            "Preview paused"
        } else {
            "Continue paused"
        }
    }
    pub fn request(&self) -> Arc<Request> {
        self.request.clone()
    }
    pub fn reason(&self) -> Option<&str> {
        self.expanded.then_some(&self.reason)
    }
    pub fn expanded(&self) -> bool {
        self.expanded
    }
}
impl Writer {
    pub(super) fn record_reading_refusal(&mut self, request: &Arc<Request>, reason: String) {
        if matches!(
            request.target,
            Target::Reading { .. } | Target::PreviewEdit(_)
        ) && request.origin.is_current(self)
            && request.checkpoint().is_ok()
            && !self.attention.is_composing()
        {
            let refusal = PassageRefusal {
                request: request.clone(),
                reason,
                expanded: false,
            };
            self.status = refusal.title().into();
            self.navigation.refusal = Some(refusal);
        }
    }
    pub(super) fn preview_edit_failed(&mut self, request: &Request) {
        if matches!(request.target, Target::PreviewEdit(_)) && request.checkpoint().is_ok() {
            self.return_to_trial_edit();
        }
    }
    pub fn reading_refusal(&self) -> Option<&PassageRefusal> {
        self.navigation
            .refusal
            .as_ref()
            .filter(|refusal| refusal.request.origin.is_current(self))
    }
    pub fn explain_reading_refusal(&mut self, request: &Arc<Request>) {
        if let Some(refusal) = &mut self.navigation.refusal
            && Arc::ptr_eq(request, &refusal.request)
        {
            refusal.expanded = !refusal.expanded;
        }
    }
}
