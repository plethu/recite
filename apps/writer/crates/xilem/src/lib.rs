mod app;
mod attention;
mod choices;
mod editor_work;
mod external_editor;
mod feedback;
mod keyed;
mod map;
mod native_host;
mod navigation;
mod presentation;
mod reading;
mod routes;
mod rules;
mod session;
#[cfg(test)]
mod tests;
mod trial;
mod view;
pub use recite_writer_session::startup::{HELP, LaunchOptions};
use xilem::masonry::kurbo;

use recite_writer_model::Workbench;
use xilem::{AppState, WindowId};

struct Writer {
    session: session::Session,
    editor: Option<recite_writer_session::startup::EditorCommand>,
    external: external_editor::ExternalEditor,
    window: WindowId,
    phase: Phase,
    status: String,
    beat_page: usize,
    file_page: usize,
    trial: trial::Trial,
    reading: reading::Reading,
    map: map::MapSession,
    attention: attention::Attention,
    work: editor_work::EditorWork,
    navigation: navigation::Navigation,
    repair: navigation::repair::Repair,
    recovery_copy: Option<std::path::PathBuf>,
    recovery_details: bool,
    location: recite_writer_session::location::Location,
    rules: Option<rules::ReplySession>,
    pending_choice: Option<choices::Request>,
    pending_search: Option<app::picker::Search>,
    viewport: kurbo::Size,
    config: recite_config::UserConfig,
    palette: recite_writer_session::palette::Palette<xilem::peniko::Color>,
}

#[derive(PartialEq)]
enum Phase {
    Writing,
    ConfirmClose,
    ConfirmExit,
    Closed,
}

impl AppState for Writer {
    fn keep_running(&self) -> bool {
        self.phase != Phase::Closed
    }
}

impl Writer {
    fn from_session(
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
        Self {
            session,
            editor,
            external: external_editor::ExternalEditor::new(),
            window: WindowId::next(),
            phase: Phase::Writing,
            status,
            beat_page: 0,
            file_page: 0,
            trial: trial::Trial::new(),
            reading,
            map: map::MapSession::new(),
            attention: Default::default(),
            work: editor_work::EditorWork::new(),
            navigation: navigation::Navigation::new(),
            repair: navigation::repair::Repair::default(),
            recovery_copy: None,
            recovery_details: false,
            location: Default::default(),
            rules: None,
            pending_choice: None,
            pending_search: None,
            viewport: kurbo::Size::new(1280., 800.),
            palette: presentation::palette(&recite_config::WriterConfig::default()),
            config: Default::default(),
        }
    }

    fn configure(&mut self, config: recite_config::UserConfig) {
        self.map
            .rescale(self.ui_scale(), presentation::scale(&config.writer));
        self.palette = presentation::palette(&config.writer);
        self.config = config;
    }
    fn ui_scale(&self) -> f32 {
        presentation::scale(&self.config.writer)
    }
    fn reading_size(&self) -> f32 {
        f32::from(
            self.config
                .writer
                .presentation
                .value(recite_config::WriterPresentationField::ReadingSize),
        )
    }
    fn script_width(&self) -> f64 {
        view::workspace::Layout::resolve(self).script
    }
    fn navigation_width(&self) -> f64 {
        f64::from(
            self.config
                .writer
                .presentation
                .value(recite_config::WriterPresentationField::DrawerWidth),
        )
    }

    fn change(
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
            Ok(()) => "Draft updated".into(),
            Err(error) => feedback::file(&error),
        };
        self.reading.prepare(self.session.model());
    }
    fn reload(&mut self) {
        self.navigate(navigation::Target::Reload);
    }
    fn request_close(&mut self) {
        if self.work.busy() || self.navigation.busy() {
            self.status = "An editor action is still finishing. Your work remains open.".into();
            return;
        }
        if self.session.dirty() {
            self.phase = Phase::ConfirmClose;
        } else if self.config.writer.confirm_exit {
            self.phase = Phase::ConfirmExit;
        } else {
            self.close_with_recovery();
        }
    }
    fn close_with_recovery(&mut self) {
        if self.work.busy() || self.navigation.busy() {
            self.status = "Wait for the current editor action before closing.".into();
            return;
        }
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

pub fn run_launch(options: LaunchOptions) -> Result<(), String> {
    let Some(app) = app::App::new(options)? else {
        return Ok(());
    };
    app::run(app).map_err(|error| error.to_string())
}
