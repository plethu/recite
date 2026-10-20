mod app;
mod editor_work;
mod feedback;
mod keyed;
mod navigation;
mod presentation;
mod reading;
mod session;
mod startup;
#[cfg(test)]
mod tests;
mod trial;
mod view;
pub use startup::{HELP, LaunchOptions, Startup};

use recite_writer_model::Workbench;
use recite_writer_session::{FileError, ProjectSession};
use std::ffi::OsString;
use xilem::{AppState, WindowId};

pub struct Writer {
    session: session::Session,
    editor: Option<OsString>,
    editor_args: Vec<OsString>,
    window: WindowId,
    phase: Phase,
    status: String,
    beat_page: usize,
    file_page: usize,
    trial: trial::Trial,
    reading: reading::Reading,
    work: editor_work::EditorWork,
    recovery_copy: Option<std::path::PathBuf>,
    recovery_details: bool,
    location: recite_writer_session::location::Location,
    rules: Option<recite_writer_model::ReplyRules>,
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
    pub fn open(startup: Startup) -> Result<Self, FileError> {
        let project = ProjectSession::open(&startup.project, startup.document.as_deref())?;
        let editor = startup
            .editor
            .map(|program| recite_writer_session::startup::EditorCommand {
                program,
                args: startup.editor_args,
            });
        Ok(Self::from_session(
            session::Session::Project(Box::new(project)),
            editor,
        ))
    }
    fn from_session(
        session: session::Session,
        editor: Option<recite_writer_session::startup::EditorCommand>,
    ) -> Self {
        let mut reading = reading::Reading::new();
        reading.prepare(session.model());
        let status = if session.dirty() {
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
            editor: editor.as_ref().map(|command| command.program.clone()),
            editor_args: editor.map_or_else(Vec::new, |command| command.args),
            window: WindowId::next(),
            phase: Phase::Writing,
            status,
            beat_page: 0,
            file_page: 0,
            trial: trial::Trial::new(),
            reading,
            work: editor_work::EditorWork::new(),
            recovery_copy: None,
            recovery_details: false,
            location: Default::default(),
            rules: None,
            palette: presentation::palette(&recite_config::WriterConfig::default()),
            config: Default::default(),
        }
    }

    fn configure(&mut self, config: recite_config::UserConfig) {
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
        f64::from(
            self.config
                .writer
                .presentation
                .value(recite_config::WriterPresentationField::ScriptWidth),
        )
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
        self.status = match self.session.edit(action) {
            Ok(()) => "Draft updated".into(),
            Err(error) => feedback::file(&error),
        };
        self.reading.prepare(self.session.model());
    }
    fn reload(&mut self) {
        if self.work.busy() {
            self.status = "Wait for the current save before reloading.".into();
            return;
        }
        self.trial.cancel();
        let result = self
            .session
            .project_mut()
            .ok_or(FileError::ProjectRequired)
            .and_then(ProjectSession::reload_current);
        self.status = match result {
            Ok(Some(path)) => {
                self.recovery_copy = Some(path);
                self.recovery_details = false;
                "Reloaded; previous work preserved in a recovery copy.".into()
            }
            Ok(None) => "Reloaded from disk".into(),
            Err(error) => feedback::file(&error),
        };
        self.reading.prepare(self.session.model());
    }
    fn open_editor(&mut self) {
        if self.work.busy() {
            self.status = "Wait for the current save before opening your editor.".into();
            return;
        }
        let Some(editor) = self.editor.clone() else {
            return;
        };
        let Some(project) = self.session.project_mut() else {
            self.status = FileError::ProjectRequired.to_string();
            return;
        };
        if let Err(error) = project.current_mut().prepare_external_edit() {
            self.status = feedback::file(&error);
            return;
        }
        self.status = match std::process::Command::new(editor)
            .args(&self.editor_args)
            .arg(project.current().path())
            .spawn()
        {
            Ok(mut child) => {
                // Reap the independently owned editor without blocking Writer's event loop.
                std::thread::spawn(move || {
                    let _ = child.wait();
                });
                "Opened accepted source in your editor; Writer drafts remain recoverable. Reload when you return.".into()
            }
            Err(error) => format!("Could not open the editor: {error}"),
        };
    }
    fn request_close(&mut self) {
        if self.work.busy() {
            self.status = "Saving is still finishing. Your work remains open.".into();
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
        if self.work.busy() {
            self.status = "Wait for the current save before closing.".into();
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

pub fn run(writer: Writer) -> Result<(), xilem::winit::error::EventLoopError> {
    app::application(app::App::with_writer(writer)).run_in(xilem::EventLoop::with_user_event())
}
pub fn run_launch(options: LaunchOptions) -> Result<(), String> {
    let Some(app) = app::App::new(options)? else {
        return Ok(());
    };
    app::application(app)
        .run_in(xilem::EventLoop::with_user_event())
        .map_err(|error| error.to_string())
}
