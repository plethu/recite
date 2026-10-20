mod keyed;
mod startup;
#[cfg(test)]
mod tests;
mod view;
pub use startup::{HELP, Startup};

use recite_writer_model::{View, Workbench};
use recite_writer_session::{FileError, ProjectSession};
use std::ffi::OsString;
use xilem::{AppState, WindowId, WindowOptions, Xilem};

pub struct Writer {
    project: ProjectSession,
    editor: Option<OsString>,
    editor_args: Vec<OsString>,
    window: WindowId,
    phase: Phase,
    status: String,
    page: usize,
    file_page: usize,
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
        let project = ProjectSession::open(&startup.project, startup.document.as_deref())?;
        let status = if project.dirty() {
            "Recovered unsaved work; review it before saving."
        } else {
            "Ready"
        }
        .into();
        Ok(Self {
            project,
            editor: startup.editor,
            editor_args: startup.editor_args,
            window: WindowId::next(),
            phase: Phase::Writing,
            status,
            page: 0,
            file_page: 0,
        })
    }

    fn change(
        &mut self,
        action: impl FnOnce(&mut Workbench) -> Result<(), recite_writer_model::WorkbenchError>,
    ) {
        self.status = match self.project.current_mut().edit(action) {
            Ok(()) => "Draft updated".into(),
            Err(error) => error.to_string(),
        };
    }
    fn save(&mut self) -> bool {
        match self.project.current_mut().save() {
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
    fn save_all(&mut self) -> bool {
        match self.project.save_all() {
            Ok(()) => {
                self.status = "All open files saved".into();
                true
            }
            Err(error) => {
                self.status = error.to_string();
                false
            }
        }
    }
    fn switch(&mut self, path: &std::path::Path) {
        self.status = match self.project.switch(path) {
            Ok(()) => {
                self.page = 0;
                if self.project.current().dirty() {
                    "Unsaved work restored"
                } else {
                    "Ready"
                }
                .into()
            }
            Err(error) => error.to_string(),
        };
    }
    fn reload(&mut self) {
        let result = self.project.reload_current();
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
        if let Err(error) = self.project.current_mut().prepare_external_edit() {
            self.status = error.to_string();
            return;
        }
        self.status = match std::process::Command::new(editor)
            .args(&self.editor_args)
            .arg(self.project.current().path())
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
        if self.project.dirty() {
            self.phase = Phase::ConfirmClose;
        } else {
            self.close_with_recovery();
        }
    }
    fn close_with_recovery(&mut self) {
        match self.project.checkpoint() {
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

fn application(
    writer: Writer,
) -> Xilem<Writer, impl FnMut(&mut Writer) -> std::iter::Once<xilem::WindowView<Writer>>> {
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
}

pub fn run(writer: Writer) -> Result<(), xilem::winit::error::EventLoopError> {
    application(writer).run_in(xilem::EventLoop::with_user_event())
}
