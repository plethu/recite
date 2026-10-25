//! A refused reading transition belongs to the exact passage/draft that requested it.
use super::*;

pub(crate) struct PassageRefusal {
    request: Arc<Request>,
    reason: String,
    expanded: bool,
}
impl PassageRefusal {
    pub(crate) fn request(&self) -> Arc<Request> {
        self.request.clone()
    }
    pub(crate) fn reason(&self) -> Option<&str> {
        self.expanded.then_some(&self.reason)
    }
    pub(crate) fn expanded(&self) -> bool {
        self.expanded
    }
}
impl Writer {
    pub(super) fn record_reading_refusal(&mut self, request: &Arc<Request>, reason: String) {
        if matches!(request.target, Target::Reading { .. })
            && request.origin.is_current(self)
            && request.checkpoint().is_ok()
            && !self.attention.is_composing()
        {
            self.navigation.refusal = Some(PassageRefusal {
                request: request.clone(),
                reason,
                expanded: false,
            });
            self.status = "Can't continue yet.".into();
        }
    }
    pub(crate) fn reading_refusal(&self) -> Option<&PassageRefusal> {
        self.navigation
            .refusal
            .as_ref()
            .filter(|refusal| refusal.request.origin.is_current(self))
    }
    pub(crate) fn explain_reading_refusal(&mut self, request: &Arc<Request>) {
        if let Some(refusal) = &mut self.navigation.refusal
            && Arc::ptr_eq(request, &refusal.request)
        {
            refusal.expanded = !refusal.expanded;
        }
    }
}
