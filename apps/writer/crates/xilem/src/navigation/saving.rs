//! One Save intent may follow explicit Apply; Save All dominates repeated Save requests.
use super::*;
use recite_writer_session::project::save::ordered::Scope;

#[derive(Clone, Copy)]
enum SaveScope {
    Current,
    All,
}
pub(super) struct DeferredSave {
    request: Arc<Request>,
    scope: SaveScope,
    applied: bool,
}
impl Writer {
    pub(crate) fn defer_save_until_apply(&mut self, scope: &Scope) -> bool {
        let scope = match scope {
            Scope::Current => SaveScope::Current,
            Scope::All => SaveScope::All,
            _ => return false,
        };
        if let Some(deferred) = &mut self.navigation.deferred_save {
            if matches!(scope, SaveScope::All) {
                deferred.scope = scope;
            }
        } else {
            let Some(request) = self
                .navigation
                .pending
                .as_ref()
                .filter(|request| matches!(request.target, Target::Edit(EditOperation::Apply)))
            else {
                return false;
            };
            self.navigation.deferred_save = Some(DeferredSave {
                request: request.clone(),
                scope,
                applied: false,
            });
        }
        self.status = "Save requested; waiting for Apply.".into();
        true
    }
    pub(super) fn mark_deferred_save_applied(&mut self, request: &Arc<Request>) {
        if let Some(deferred) = &mut self.navigation.deferred_save
            && Arc::ptr_eq(request, &deferred.request)
        {
            deferred.applied = true;
        }
    }
    pub(crate) fn navigation_retired(&mut self, request: &Arc<Request>) {
        if self
            .navigation
            .deferred_save
            .as_ref()
            .is_none_or(|deferred| !deferred.applied || !Arc::ptr_eq(request, &deferred.request))
        {
            return;
        }
        if let Some(deferred) = self.navigation.deferred_save.take() {
            match deferred.scope {
                SaveScope::Current => self.save(),
                SaveScope::All => self.save_all(),
            }
        }
    }
}
