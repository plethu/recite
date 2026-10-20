mod startup;
#[cfg(test)]
mod tests;
mod view;
pub use startup::{HELP, Startup};

use recite_writer_model::{View, Workbench};
use recite_writer_session::{DocumentSession, FileError, load_context};
use std::{ffi::OsString, path::PathBuf};
use xilem::{AppState, WindowId, WindowOptions, Xilem};

pub struct Writer {
    session: DocumentSession,
    project: PathBuf,
    editor: Option<OsString>,
    editor_args: Vec<OsString>,
    window: WindowId,
    phase: Phase,
    status: String,
    page: usize,
}

#[derive(PartialEq)]
enum Phase {
    Writing,
    ConfirmClose,
    Closed,
}

impl AppState for Writer {
    fn keep_running(&self) -> bool {
        self.phase != Phase::Closed
    }
}

impl Writer {
    pub fn open(startup: Startup) -> Result<Self, FileError> {
        let report = recite_config::discover_project(&startup.project)?;
        if !report.is_complete() {
            return Err(FileError::Incomplete);
        }
        let root = report.manifest().project_root().to_owned();
        let requested = startup.document.map(|path| root.join(path));
        let document = report
            .documents()
            .iter()
            .find(|document| {
                requested
                    .as_ref()
                    .is_none_or(|path| document.path() == path)
            })
            .ok_or(FileError::Selection)?;
        let session = DocumentSession::open(
            document.path().to_owned(),
            document.key().clone(),
            load_context(&report)?,
        )?;
        let status = if session.dirty() {
            "Recovered unsaved work; review it before saving."
        } else {
            "Ready"
        }
        .into();
        Ok(Self {
            session,
            project: root,
            editor: startup.editor,
            editor_args: startup.editor_args,
            window: WindowId::next(),
            phase: Phase::Writing,
            status,
            page: 0,
        })
    }

    fn change(
        &mut self,
        action: impl FnOnce(&mut Workbench) -> Result<(), recite_writer_model::WorkbenchError>,
    ) {
        self.status = match self.session.edit(action) {
            Ok(()) => "Draft updated".into(),
            Err(error) => error.to_string(),
        };
    }
    fn save(&mut self) -> bool {
        match self.session.save() {
            Ok(()) => {
                self.status = "Saved".into();
                true
            }
            Err(error) => {
                self.status = error.to_string();
                false
            }
        }
    }
    fn reload(&mut self) {
        let result = recite_config::discover_project(&self.project)
            .map_err(FileError::from)
            .and_then(|report| {
                if !report.is_complete() {
                    return Err(FileError::Incomplete);
                }
                self.session.reload(load_context(&report)?)
            });
        self.status = match result {
            Ok(Some(path)) => format!("Reloaded; previous work preserved at {}", path.display()),
            Ok(None) => "Reloaded from disk".into(),
            Err(error) => error.to_string(),
        };
    }
    fn open_editor(&mut self) {
        let Some(editor) = self.editor.clone() else {
            return;
        };
        if let Err(error) = self.session.prepare_external_edit() {
            self.status = error.to_string();
            return;
        }
        self.status = match std::process::Command::new(editor)
            .args(&self.editor_args)
            .arg(self.session.path())
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
        if self.session.dirty() {
            self.phase = Phase::ConfirmClose;
        } else {
            self.close_with_recovery();
        }
    }
    fn close_with_recovery(&mut self) {
        match self.session.checkpoint() {
            Ok(()) => self.phase = Phase::Closed,
            Err(error) => {
                self.status = error.to_string();
                self.phase = Phase::ConfirmClose;
            }
        }
    }
    fn select(&mut self, id: String) {
        self.change(|model| model.select(View::Passage(id)));
    }
}

pub fn run(writer: Writer) -> Result<(), xilem::winit::error::EventLoopError> {
    Xilem::new(writer, |state: &mut Writer| {
        std::iter::once(
            xilem::window(state.window, "Recite Writer", view::writer(state)).with_options(|_| {
                WindowOptions::new("Recite Writer")
                    .with_initial_inner_size(xilem::winit::dpi::LogicalSize::new(1200., 800.))
                    .with_min_inner_size(xilem::winit::dpi::LogicalSize::new(900., 650.))
                    .on_close(Writer::request_close)
            }),
        )
    })
    .with_font(include_bytes!("../../freya/fonts/literata/Literata-Variable.ttf").to_vec())
    .run_in(xilem::EventLoop::with_user_event())
}
