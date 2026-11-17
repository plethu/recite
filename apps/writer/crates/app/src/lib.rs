//! Toolkit-independent Writer state, workflows and policy.
//!
//! The native frontend renders this state and forwards input to it. Nothing here depends on a
//! GUI toolkit, so a frontend can change without moving authoring behaviour.
pub mod attention;
mod closing;
mod host;
pub use host::Host;
mod choices;
mod intents;
pub use choices::Opener;
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
mod window;
pub use window::{COMPACT_WINDOW, NARROW_SIDEBAR, WindowKey};
pub mod workspaces;
use recite_writer_model::Workbench;

pub struct Writer {
    pub(crate) host: Host,
    pub session: session::Session,
    pub editor: Option<recite_writer_session::startup::EditorCommand>,
    pub external: external_editor::ExternalEditor,
    pub window: WindowKey,
    pub(crate) phase: Phase,
    pub status: String,
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
    /// The label whose choice slip is open.
    pub(crate) choosing: Option<choices::Opener>,
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
            "Open the source in your editor, then Reload."
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
            choosing: None,
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
}
