//! Reload keeps the live lease and adopts only after a complete independent archive exists.
use super::*;
use recite_writer_session::project::PreparedReload;

impl Writer {
    pub(super) fn submit_reload(&mut self) -> Result<(), String> {
        let project = self.session.project().ok_or("Open a project first.")?;
        let reload = project
            .files()
            .reload_request(project.current().model())
            .map_err(|error| feedback::file(&error))?;
        let request = Arc::new(Request {
            target: Target::Reload,
            origin: Origin::capture(self),
            cancel: CancellationToken::new(),
            forwarded: None,
        });
        self.send_navigation(request, worker::Work::Reload(Box::new(reload)))
    }
    pub(super) fn record_reload_archive(&mut self, prepared: &PreparedReload) {
        self.recovery_copy = Some(prepared.archive().to_owned());
        self.recovery_details = false;
    }
    pub(super) fn publish_reload(
        &mut self,
        request: &Request,
        prepared: &mut PreparedReload,
    ) -> Result<(), String> {
        if !request.origin.is_current(self) {
            return Err("The document changed while reload was preparing. Your current work stays open; the captured draft has a recovery copy.".into());
        }
        self.session
            .project_mut()
            .ok_or("The project closed while reload was preparing.")?
            .accept_reload(prepared)
            .map_err(|error| feedback::file(&error))?;
        self.retire_rules();
        self.beat_page = 0;
        self.file_page = 0;
        self.reading.prepare(self.session.model());
        self.sync_map_selection();
        self.invalidate_map();
        self.remember_location();
        self.status = "Reloaded; previous work preserved in a recovery copy.".into();
        Ok(())
    }
}
