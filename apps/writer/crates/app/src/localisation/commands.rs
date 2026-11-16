//! The Localise screen's commands: opening it, catalogue actions and their feedback, search,
//! inspection and settling an external catalogue change.
use super::*;

impl Writer {
    pub fn open_localisation(&mut self) {
        if !self.can_open_localisation() {
            return;
        }
        if self.localisation.has_catalogue() {
            self.open_translation_beat(0);
        } else {
            self.enter_localisation();
        }
    }
    fn can_open_localisation(&self) -> bool {
        self.phase == Phase::Writing
            && self.session.project().is_some()
            && !self.project_action_blocked()
    }
    pub(crate) fn enter_localisation(&mut self) -> bool {
        if !self.can_open_localisation() {
            return false;
        }
        self.attention.leave_focus();
        self.cancel_map_gesture();
        self.remember_location();
        self.workspaces.screen = Screen::Localisation;
        self.remember_catalogue_location();
        true
    }
    pub fn catalogue_feedback(&mut self, message: String) {
        self.status = message
            .lines()
            .next()
            .unwrap_or("Catalogue action finished")
            .chars()
            .take(240)
            .collect();
        self.localisation.report = if message.contains('\n') || message.chars().count() > 240 {
            text::pages(&message)
        } else {
            Vec::new()
        };
        self.localisation.report_page = 0;
    }
    pub fn catalogue_action(&mut self, action: work::Action) {
        let closing = matches!(
            action,
            work::Action::FlushForClose | work::Action::DiscardForClose
        );
        if self.phase == Phase::Closed
            || (!closing && (self.phase != Phase::Writing || self.localisation.slot.read_only()))
            || (self.session.read_only() && !closing)
            || self.navigation.busy()
            || self.work.busy()
            || self.project_jobs_busy()
        {
            return;
        }
        let selected = self.localisation.mounted_entries();
        match self.localisation.slot.checkout(selected) {
            Ok(checkout) => self.submit_catalogue_tool(Work::checked(
                checkout,
                action,
                self.localisation.query.clone(),
                self.session.model().document().cached_passages(),
            )),
            Err(error) => self.catalogue_feedback(error),
        }
    }
    pub fn open_catalogue_path(&mut self) {
        let path = PathBuf::from(self.localisation.path.trim());
        if !path.as_os_str().is_empty() {
            self.catalogue_action(work::Action::Open(path));
        }
    }
    pub(crate) fn flush_catalogue_before_close(&mut self) {
        self.catalogue_action(work::Action::FlushForClose);
    }
    pub fn search_catalogue(&mut self) {
        self.catalogue_action(work::Action::Query);
    }
    pub fn inspect_translation(&mut self, key: &EntryKey) {
        if self.localisation.slot.identity() == Some(&key.catalogue) {
            self.catalogue_action(work::Action::Inspect(key.id));
        }
    }
    pub fn cancel_catalogue_action(&mut self) {
        self.localisation.slot.cancel();
        self.session.cancel_exclusive();
        self.status = "Cancellation requested; waiting for the catalogue owner to return.".into();
    }
    pub fn compare_catalogue(&mut self) {
        self.catalogue_action(work::Action::Compare);
    }
    pub fn resolve_catalogue(&mut self, decision: ExternalDecision) {
        if let Some(comparison) = &self.localisation.comparison {
            self.catalogue_action(work::Action::Resolve {
                decision,
                inspection: Box::new(comparison.inspection.clone()),
            });
        }
    }
}
