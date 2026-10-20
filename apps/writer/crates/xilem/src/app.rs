//! Window ownership, welcome and cold project loading live outside active authoring sessions.
#[cfg(target_os = "linux")]
mod activation;
mod opening;
mod settings;
mod slot;
mod view;
use super::{Phase, Writer, session::Session};
use opening::{Job, Opening};
use recite_compiler::authoring::CancellationToken;
use recite_writer_session::{
    activation::{Activation, ActivationHost, ActivationInbox},
    startup::{LaunchOptions, Startup},
};
use std::{
    path::PathBuf,
    sync::{Arc, mpsc},
};
use tokio::sync::mpsc::UnboundedSender;
use xilem::{AppState, WindowId};

pub(super) struct App {
    writer: Option<Writer>,
    preferences: recite_writer_session::preferences::Preferences,
    theme: crate::presentation::Theme,
    settings_open: bool,
    settings: settings::Settings,
    window: WindowId,
    show_welcome: bool,
    closed: bool,
    path: String,
    native_path: Option<PathBuf>,
    name: String,
    creating: bool,
    auxiliary: bool,
    status: String,
    initial: Option<LaunchOptions>,
    opening: Option<Opening>,
    sender: Option<UnboundedSender<Job>>,
    _host: Option<ActivationHost>,
    inbox: Option<ActivationInbox>,
}
impl AppState for App {
    fn keep_running(&self) -> bool {
        !self.closed && self.writer.as_ref().is_none_or(AppState::keep_running)
    }
}
impl App {
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
        app.theme.refresh(&app.preferences.config.writer);
        Ok(Some(app))
    }
    fn empty() -> Self {
        Self {
            writer: None,
            preferences: recite_writer_session::preferences::Preferences::load(None),
            theme: crate::presentation::Theme::new(&recite_config::WriterConfig::default()),
            settings_open: false,
            settings: Default::default(),
            window: WindowId::next(),
            show_welcome: true,
            closed: false,
            path: String::new(),
            native_path: None,
            name: String::new(),
            creating: false,
            auxiliary: false,
            status: "Open a project or create one to start writing.".into(),
            initial: None,
            opening: None,
            sender: None,
            _host: None,
            inbox: None,
        }
    }
    pub fn with_writer(writer: Writer) -> Self {
        Self {
            window: writer.window,
            writer: Some(writer),
            show_welcome: false,
            status: String::new(),
            ..Self::empty()
        }
    }
    fn can_replace(&self) -> bool {
        self.writer.as_ref().is_none_or(|writer| {
            !writer.work.busy()
                && match &writer.session {
                    Session::Project(project) => project.files().can_leave(writer.session.model()),
                    Session::Examples { .. } => !writer.session.dirty(),
                }
        })
    }
    fn request_close(&mut self) {
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
    fn open_selected(&mut self) {
        if self.opening.is_some() || self.auxiliary {
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
                    self.auxiliary = true;
                    self.status = "Creating project…".into();
                }
                Err(error) => self.status = error,
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
                self.status = error;
            }
        }
    }
    fn browse(&mut self) {
        if self.auxiliary || self.opening.is_some() {
            return;
        }
        match self.send(Job::Browse) {
            Ok(()) => self.auxiliary = true,
            Err(error) => self.status = error,
        }
    }
    fn show_examples(&mut self) {
        let options = LaunchOptions {
            intent: Startup::Examples { route: None },
            document: None,
            editor: None,
        };
        if let Err(error) = self.start_open(options, None, None) {
            self.status = error;
        }
    }
    fn start_open(
        &mut self,
        options: LaunchOptions,
        reply: Option<mpsc::Sender<Result<(), String>>>,
        cancelled: Option<Arc<std::sync::atomic::AtomicBool>>,
    ) -> Result<(), String> {
        if self.opening.is_some() || self.auxiliary {
            return Err("Finish the current project action before opening another project.".into());
        }
        if !self.can_replace() {
            return Err("Return to Writer and save changes before opening another project. Example edits must be discarded explicitly.".into());
        }
        let request = Arc::new(opening::Request {
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
    fn cancel_open(&mut self) {
        if let Some(opening) = &self.opening {
            opening.request.cancel.interrupt();
            self.status = "Cancelling project opening…".into();
        }
    }
}
pub(super) fn application(
    app: App,
) -> xilem::Xilem<App, impl FnMut(&mut App) -> std::iter::Once<xilem::WindowView<App>>> {
    let defaults = app.theme.initial_properties();
    xilem::Xilem::new(app, |app: &mut App| {
        std::iter::once(
            xilem::window(app.window, "Recite Writer", view::app(app))
                .with_default_properties(app.theme.properties.clone())
                .with_base_color(app.theme.palette.canvas)
                .with_options(|_| {
                    xilem::WindowOptions::new("Recite Writer")
                        .with_initial_inner_size(xilem::winit::dpi::LogicalSize::new(1200., 800.))
                        .with_min_inner_size(xilem::winit::dpi::LogicalSize::new(900., 650.))
                        .on_close(App::request_close)
                }),
        )
    })
    .with_default_properties(defaults)
    .with_font(include_bytes!("../../freya/fonts/literata/Literata-Variable.ttf").to_vec())
}
