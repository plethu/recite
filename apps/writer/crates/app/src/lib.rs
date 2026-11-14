//! Toolkit-independent Writer state, workflows and policy.
//!
//! The native frontend renders this state and forwards input to it. Nothing here depends on a
//! GUI toolkit, so a frontend can change without moving authoring behaviour.
pub mod attention;
mod choices;
pub mod editor_work;
pub mod external_editor;
pub mod feedback;
pub mod localisation;
mod lookup;
pub mod map;
pub mod navigation;
pub mod notices;
pub mod presentation;
pub mod reading;
mod reading_context;
pub mod routes;
pub mod rules;
pub mod session;
pub mod shell;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
pub mod trial;
pub mod workspaces;
use recite_writer_model::Workbench;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

/// Identifies one Writer window so late asynchronous results can be matched to their origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WindowKey(NonZeroU64);
impl WindowKey {
    /// Allocate a key that no other window in this process uses.
    pub fn next() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        Self(NonZeroU64::new(id).unwrap_or(NonZeroU64::MIN))
    }
}

/// Native services a frontend lends to authoring workflows.
#[derive(Clone, Copy)]
pub struct Host {
    /// Ask the person for a repaired source file; runs on a worker thread.
    pub choose_repaired_source: fn() -> Option<std::path::PathBuf>,
}
impl Default for Host {
    fn default() -> Self {
        Self {
            choose_repaired_source: || None,
        }
    }
}

pub struct Writer {
    pub(crate) host: Host,
    pub session: session::Session,
    pub editor: Option<recite_writer_session::startup::EditorCommand>,
    pub external: external_editor::ExternalEditor,
    pub window: WindowKey,
    pub(crate) phase: Phase,
    pub status: String,
    pub beat_page: usize,
    pub file_page: usize,
    pub trial: trial::Trial,
    pub reading: reading::Reading,
    pub map: map::MapSession,
    pub attention: attention::Attention,
    pub work: editor_work::EditorWork,
    pub navigation: navigation::Navigation,
    pub repair: navigation::repair::Repair,
    pub recovery_copy: Option<std::path::PathBuf>,
    pub recovery_details: bool,
    location: recite_writer_session::location::Location,
    pub rules: Option<rules::ReplySession>,
    pending_choice: Option<choices::Request>,
    pub(crate) app_request: Option<shell::Request>,
    pub pending_clipboard: Option<String>,
    lookup: Option<lookup::Search>,
    pub workspaces: workspaces::Workspaces,
    pub localisation: localisation::Localisation,
    pub notices: notices::Notices,
    pub viewport: kurbo::Size,
    pub(crate) document_menu: attention::DocumentMenu,
    pub reading_context: reading_context::Context,
    pub config: recite_config::UserConfig,
    pub palette: recite_writer_session::palette::Palette<presentation::Color>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Writing,
    ConfirmClose,
    ConfirmExit,
    Closing,
    Closed,
}

impl Writer {
    /// Whether the window should stay open.
    pub fn keep_running(&self) -> bool {
        self.phase != Phase::Closed
    }
    pub fn from_session(
        session: session::Session,
        editor: Option<recite_writer_session::startup::EditorCommand>,
    ) -> Self {
        let mut reading = reading::Reading::new();
        reading.prepare(session.model());
        let status = if session.model().view() == &recite_writer_model::View::Source
            && session.model().has_draft()
        {
            "Recovered source draft awaiting repair and import."
        } else if session.dirty() {
            "Recovered unsaved work; review it before saving."
        } else if session.project().is_none() {
            "Temporary examples. Edits stay in this window and are discarded when it closes."
        } else if reading.script().is_err() || reading.error.is_some() {
            "Source needs attention. Open it in your editor, then reload."
        } else {
            "Ready"
        }
        .into();
        let mut writer = Self {
            host: Host::default(),
            session,
            editor,
            external: external_editor::ExternalEditor::new(),
            window: WindowKey::next(),
            phase: Phase::Writing,
            status,
            beat_page: 0,
            file_page: 0,
            trial: trial::Trial::new(),
            reading,
            map: map::MapSession::new(),
            attention: Default::default(),
            work: editor_work::EditorWork::new(),
            navigation: navigation::Navigation::default(),
            repair: navigation::repair::Repair::default(),
            recovery_copy: None,
            recovery_details: false,
            location: Default::default(),
            rules: None,
            pending_choice: None,
            app_request: None,
            pending_clipboard: None,
            lookup: None,
            workspaces: Default::default(),
            localisation: Default::default(),
            notices: Default::default(),
            viewport: kurbo::Size::new(1280., 800.),
            document_menu: Default::default(),
            reading_context: reading_context::Context::default(),
            palette: presentation::palette(&recite_config::WriterConfig::default()),
            config: Default::default(),
        };
        writer.reveal_navigation();
        writer
    }

    pub fn configure(&mut self, config: recite_config::UserConfig) {
        if self.ui_scale() != presentation::scale(&config.writer) {
            self.map.cancel_gestures();
        }
        self.palette = presentation::palette(&config.writer);
        self.config = config;
    }
    pub fn ui_scale(&self) -> f32 {
        presentation::scale(&self.config.writer)
    }
    pub fn reading_size(&self) -> f32 {
        f32::from(
            self.config
                .writer
                .presentation
                .value(recite_config::WriterPresentationField::ReadingSize),
        )
    }
    pub fn compact_chrome(&self) -> bool {
        self.viewport.width < 900. * f64::from(self.ui_scale())
    }
    pub fn content_width(&self) -> f64 {
        (self.viewport.width - self.navigation_width() - 32.).max(280.)
    }
    pub fn navigation_width(&self) -> f64 {
        if self.attention.is_focused() || self.viewport.width < 900. * f64::from(self.ui_scale()) {
            return 0.;
        }
        self.drawer_width()
    }
    pub fn drawer_width(&self) -> f64 {
        f64::from(
            self.config
                .writer
                .presentation
                .value(recite_config::WriterPresentationField::DrawerWidth),
        )
    }

    pub fn change(
        &mut self,
        action: impl FnOnce(&mut Workbench) -> Result<(), recite_writer_model::WorkbenchError>,
    ) {
        let result = self.session.edit(action);
        if result.is_ok()
            && self.rules.as_ref().is_some_and(|rules| {
                self.session.model().view() != &recite_writer_model::View::Source
                    || !rules
                        .projection()
                        .belongs_to(self.session.model().document())
            })
        {
            self.retire_rules();
        }
        self.status = match result {
            Ok(()) if self.source_repair_pending() => {
                "Recovered source draft awaiting repair and import.".into()
            }
            Ok(()) => "Ready".into(),
            Err(error) => feedback::file(&error),
        };
        self.reading.prepare(self.session.model());
    }
    pub fn reload(&mut self) {
        self.navigate(navigation::Target::Reload);
    }
    pub fn request_close(&mut self) {
        if let Err(error) = self.require_finished_composition() {
            self.status = error;
            return;
        }
        if self.work.busy() || self.navigation.busy() || self.project_jobs_busy() {
            self.status = "An editor action is still finishing. Your work remains open.".into();
            return;
        }
        if self.session.dirty() || self.localisation.dirty() {
            self.phase = Phase::ConfirmClose;
        } else if self.config.writer.confirm_exit {
            self.phase = Phase::ConfirmExit;
        } else {
            self.close_with_recovery();
        }
    }
    pub fn close_with_recovery(&mut self) {
        if self.work.busy() || self.navigation.busy() || self.project_jobs_busy() {
            self.status = "Wait for the current editor action before closing.".into();
            return;
        }
        if self.localisation.has_catalogue() {
            self.phase = Phase::Closing;
            self.flush_catalogue_before_close();
            return;
        }
        self.close_sources_with_recovery();
    }
    pub(crate) fn close_sources_with_recovery(&mut self) {
        let Some(project) = self.session.project_mut() else {
            self.phase = Phase::Closed;
            return;
        };
        match project.checkpoint() {
            Ok(()) => self.phase = Phase::Closed,
            Err(error) => {
                self.status = feedback::file(&error);
                self.phase = Phase::ConfirmClose;
            }
        }
    }
}

impl Writer {
    /// The open document and whether it has unsaved edits.
    pub fn document_caption(&self) -> String {
        format!(
            "{}{}",
            self.session.model().document().key(),
            if self.session.current_dirty() {
                " · Unsaved"
            } else {
                ""
            }
        )
    }
}

/// Intents a frontend raises from controls; each keeps related state consistent.
impl Writer {
    /// Leave a close or exit prompt and continue working.
    pub fn keep_editing(&mut self) {
        self.phase = Phase::Writing;
        self.work.keep_editing();
        self.localisation.keep_editing();
    }
    /// Leave a close prompt to review translations that still have unsaved drafts.
    pub fn open_unsaved_translations(&mut self) {
        self.keep_editing();
        self.enter_localisation();
        self.localisation.query.filter = localisation::Filter::Unsaved;
        self.search_catalogue();
    }
    /// Ask the shell to open a search.
    pub fn request_search(&mut self, search: shell::Search) {
        self.app_request = Some(shell::Request::Search(search));
    }
    /// Ask the shell to show the project welcome screen.
    pub fn request_projects(&mut self) {
        self.app_request = Some(shell::Request::Projects);
    }
    /// Ask the shell to open settings.
    pub fn request_settings(&mut self) {
        self.app_request = Some(shell::Request::Settings);
    }
    /// Close document actions after a chosen action.
    pub fn close_document_menu(&mut self) {
        self.document_menu = attention::DocumentMenu::Closed;
    }
    /// Open or close document actions from their toolbar control.
    pub fn toggle_document_menu(&mut self) {
        self.document_menu.toggle();
    }
    /// Dismiss document actions without choosing one.
    pub fn dismiss_document_menu(&mut self) {
        self.document_menu = attention::DocumentMenu::Dismissed;
    }
    /// Report that a copied link reached the system clipboard.
    pub fn clipboard_written(&mut self) {
        self.status = "Link sent to clipboard.".into();
    }
    /// Resume background work that a native message may have unblocked.
    pub fn after_native_message(&mut self) {
        self.refresh_map_if_needed();
        self.resume_external_notices();
        self.resume_linked_editor();
    }
}

/// Read access to state whose changes go through named operations.
impl Writer {
    /// Whether the window is writing, confirming a close or exit, or closing.
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn document_menu(&self) -> attention::DocumentMenu {
        self.document_menu
    }
}
