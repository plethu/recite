//! The window-independent application shell: welcome, project opening, preferences, the picker
//! and command routing live outside active authoring sessions and outside any GUI toolkit.
#[cfg(target_os = "linux")]
mod activation;
mod commands;
pub mod keyboard;
pub mod opening;
pub mod picker;
pub mod settings;
pub mod welcome;
pub use crate::{Host, Phase, Writer, session::Session};
use kurbo::Size;
pub use opening::{Job, Opening};
use recite_compiler::authoring::CancellationToken;
use recite_writer_session::{
    activation::{Activation, ActivationHost, ActivationInbox},
    startup::{LaunchOptions, Startup},
};
use std::sync::atomic;
use std::{
    path::PathBuf,
    sync::{Arc, mpsc},
};
use tokio::sync::mpsc::UnboundedSender;

/// Work the shell performs outside the authoring session.
pub enum Request {
    Search(Search),
    Workspace,
}

/// The search surfaces the shell can open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Search {
    Commands,
    GoTo,
    ProjectText,
    Passages,
}

/// Shell state shared by every frontend; the frontend adds only its window and theme.
pub struct Shell {
    pub host: Host,
    pub writer: Option<Writer>,
    pub preferences: recite_writer_session::preferences::Preferences,
    pub settings_open: bool,
    pub settings: settings::Settings,
    pub picker: Option<picker::Picker>,
    pub show_welcome: bool,
    pub(crate) closed: bool,
    pub path: String,
    pub native_path: Option<PathBuf>,
    pub name: String,
    pub creating: bool,
    pub welcome_problem: Option<welcome::Problem>,
    pub auxiliary: bool,
    pub status: String,
    pub pane_hint: bool,
    pub workspace_menu: bool,
    pub viewport: Size,
    pub(crate) initial: Option<LaunchOptions>,
    pub opening: Option<Opening>,
    pub sender: Option<UnboundedSender<Job>>,
    pub(crate) _host: Option<ActivationHost>,
    pub(crate) inbox: Option<ActivationInbox>,
}
impl Shell {
    pub fn new(options: LaunchOptions) -> Result<Option<Self>, String> {
        let host = if matches!(options.intent, Startup::Writer { .. }) {
            match Activation::claim_launch(&options)? {
                Activation::Forwarded => return Ok(None),
                Activation::Owner(host) => Some(host),
            }
        } else {
            None
        };
        let welcome = matches!(options.intent, Startup::Writer { project: None, .. });
        let inbox = host.as_ref().map(ActivationHost::inbox);
        let mut app = Self {
            initial: (!welcome).then_some(options),
            _host: host,
            inbox,
            ..Self::empty()
        };
        app.preferences = recite_writer_session::preferences::Preferences::load(Some(
            recite_config::UserConfigStore::discover().map_err(|error| error.to_string()),
        ));
        Ok(Some(app))
    }
    pub fn empty() -> Self {
        Self {
            host: Host::default(),
            writer: None,
            preferences: recite_writer_session::preferences::Preferences::load(None),
            settings_open: false,
            settings: Default::default(),
            picker: None,
            show_welcome: true,
            closed: false,
            path: String::new(),
            native_path: None,
            name: String::new(),
            creating: false,
            welcome_problem: None,
            auxiliary: false,
            status: String::new(),
            pane_hint: false,
            workspace_menu: false,
            viewport: Size::new(1280., 800.),
            initial: None,
            opening: None,
            sender: None,
            _host: None,
            inbox: None,
        }
    }
    fn can_replace(&self) -> bool {
        self.writer.as_ref().is_none_or(|writer| {
            writer.phase == Phase::Writing
                && !writer.work.busy()
                && writer.localisation.can_leave()
                && !writer.navigation.busy()
                && !writer.project_jobs_busy()
                && !writer.attention.is_composing()
                && match &writer.session {
                    Session::Project(project) => project.files().can_leave(writer.session.model()),
                    Session::Examples { .. } => !writer.session.dirty(),
                    Session::Working { .. } | Session::Recovering { .. } => false,
                }
        })
    }
    pub fn request_close(&mut self) {
        if let Some(writer) = self.writer.as_mut() {
            self.show_welcome = false;
            self.settings_open = false;
            writer.request_close();
        } else {
            if let Some(opening) = &self.opening {
                opening.request.cancel.interrupt();
            }
            self.closed = true;
        }
    }
    pub fn open_selected(&mut self) {
        if !self.project_form_ready() {
            return;
        }
        let path = self
            .native_path
            .clone()
            .unwrap_or_else(|| PathBuf::from(self.path.trim()));
        if self.creating {
            if !self.can_replace() {
                self.status =
                    "Return to Writer and save changes before creating another project.".into();
                return;
            }
            match self.send(Job::Create {
                path,
                name: self.name.trim().to_owned(),
            }) {
                Ok(()) => {
                    self.welcome_problem = None;
                    self.auxiliary = true;
                    self.status = "Creating project…".into();
                }
                Err(error) => self.project_failed(welcome::ProjectAction::Create, error),
            }
        } else {
            let options = LaunchOptions {
                intent: Startup::Writer {
                    project: Some(path),
                    route: None,
                },
                document: None,
                editor: None,
            };
            if let Err(error) = self.start_open(options, None, None) {
                self.project_failed(welcome::ProjectAction::Open, error);
            }
        }
    }
    pub fn project_form_ready(&self) -> bool {
        self.opening.is_none()
            && !self.auxiliary
            && !self.path.trim().is_empty()
            && (!self.creating || !self.name.trim().is_empty())
    }
    pub fn browse(&mut self) {
        if self.auxiliary || self.opening.is_some() {
            return;
        }
        match self.send(Job::Browse) {
            Ok(()) => self.auxiliary = true,
            Err(error) => self.status = error,
        }
    }
    pub fn show_examples(&mut self) {
        let options = LaunchOptions {
            intent: Startup::Examples { route: None },
            document: None,
            editor: None,
        };
        if let Err(error) = self.start_open(options, None, None) {
            self.status = error;
        }
    }
    pub fn start_open(
        &mut self,
        options: LaunchOptions,
        reply: Option<mpsc::Sender<Result<(), String>>>,
        cancelled: Option<Arc<atomic::AtomicBool>>,
    ) -> Result<(), String> {
        if self.opening.is_some() || self.auxiliary {
            return Err("Finish the current project action before opening another project.".into());
        }
        if !self.can_replace() {
            return Err("Return to Writer and save changes before opening another project. Example edits must be discarded explicitly.".into());
        }
        let request = Arc::new(opening::Request {
            host: self.host,
            options,
            cancel: CancellationToken::new(),
            forwarded: cancelled,
        });
        let origin = self
            .writer
            .as_ref()
            .map(|writer| writer.session.model().navigation_request());
        self.send(Job::Open(request.clone()))?;
        self.opening = Some(Opening {
            request,
            origin,
            reply,
        });
        self.welcome_problem = None;
        self.status = "Opening project…".into();
        Ok(())
    }
    fn send(&mut self, job: Job) -> Result<(), String> {
        self.sender
            .as_ref()
            .ok_or("The project worker is unavailable. Reopen Writer to retry.")?
            .send(job)
            .map_err(|_| "The project worker stopped. Reopen Writer to retry.".into())
    }
    pub fn cancel_open(&mut self) {
        if let Some(opening) = &self.opening {
            opening.request.cancel.interrupt();
            self.status = "Cancelling project opening…".into();
        }
    }
    /// Whether the application should keep its window open.
    pub fn keep_running(&self) -> bool {
        !self.closed && self.writer.as_ref().is_none_or(Writer::keep_running)
    }
}

/// Intents raised by shell controls; each keeps overlays and menus consistent.
impl Shell {
    pub fn toggle_workspace_menu(&mut self) {
        self.workspace_menu = !self.workspace_menu;
    }
    /// Leave the project welcome screen for the open document.
    pub fn return_to_document(&mut self) {
        self.show_welcome = false;
    }
    pub fn close_settings(&mut self) {
        self.settings_open = false;
    }
    /// Switch between the open document and the project welcome screen.
    pub fn toggle_projects(&mut self) {
        self.show_welcome = !self.show_welcome;
        self.settings_open = false;
        self.workspace_menu = false;
    }
    pub fn settings_from_menu(&mut self) {
        self.workspace_menu = false;
        self.show_settings();
    }
    pub fn search_from_menu(&mut self, search: Search) {
        self.workspace_menu = false;
        self.open_picker(search);
    }
    pub fn localise_from_menu(&mut self) {
        self.workspace_menu = false;
        if let Some(writer) = &mut self.writer {
            writer.open_localisation();
        }
    }
    /// Choose between opening an existing project and creating one.
    pub fn choose_project_action(&mut self, creating: bool) {
        self.creating = creating;
        self.clear_project_problem();
    }
    /// A typed folder replaces any natively chosen one.
    pub fn edit_project_path(&mut self, path: String) {
        self.clear_project_problem();
        self.path = path;
        self.native_path = None;
    }
    pub fn edit_project_name(&mut self, name: String) {
        self.name = name;
        self.clear_project_problem();
    }
    /// Act on whatever the open Writer asked of the shell during its last message.
    pub fn apply_writer_requests(&mut self) {
        let request = self
            .writer
            .as_mut()
            .and_then(|writer| writer.app_request.take());
        self.open_editor_choices();
        match request {
            Some(Request::Search(search)) => self.open_picker(search),
            Some(Request::Workspace) => self.toggle_workspace_menu(),
            None => {}
        }
    }
}

#[cfg(test)]
mod tests;
