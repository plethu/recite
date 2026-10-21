//! A separate production-component specimen; no project, recovery or durable preference owner.
mod view;
use crate::{map::Workspace, presentation::Theme};
use recite_config::{WriterConfig, WriterPresentationField, WriterTheme};
use xilem::{AppState, WindowId, winit::dpi};

struct Specimen {
    window: WindowId,
    config: WriterConfig,
    theme: Theme,
    closed: bool,
    message: String,
    query: String,
    manuscript: String,
    destination: String,
    workspace: Workspace,
    reviewed: bool,
}
impl Default for Specimen {
    fn default() -> Self {
        let config = WriterConfig::default();
        Self {
            window: WindowId::next(),
            theme: Theme::new(&config),
            config,
            closed: false,
            message: "Try the controls, then compare appearances and text sizes.".into(),
            query: String::new(),
            manuscript: "The last tram waits by the harbour. Who is coming with us?".into(),
            destination: "Missing Courier".into(),
            workspace: Workspace::Script,
            reviewed: false,
        }
    }
}
impl AppState for Specimen {
    fn keep_running(&self) -> bool {
        !self.closed
    }
}
impl Specimen {
    fn scale(&self) -> f32 {
        crate::presentation::scale(&self.config)
    }
    fn appearance(&mut self) {
        self.config.theme = if self.config.theme == WriterTheme::Dark {
            WriterTheme::Light
        } else {
            WriterTheme::Dark
        };
        self.theme.refresh(&self.config);
    }
    fn monochrome(&mut self) {
        self.config.monochrome = !self.config.monochrome;
        self.theme.refresh(&self.config);
    }
    fn scale_interface(&mut self) {
        let field = WriterPresentationField::UiScale;
        let scale = if self.config.presentation.value(field) == 100 {
            200
        } else {
            100
        };
        if let Ok(presentation) = self.config.presentation.with_value(field, scale) {
            self.config.presentation = presentation;
            self.theme.refresh(&self.config);
        }
    }
}
pub(super) fn run() -> Result<(), String> {
    let state = Specimen::default();
    let defaults = state.theme.initial_properties();
    let application_defaults = state.theme.initial_properties();
    let application = xilem::Xilem::new(state, |state: &mut Specimen| {
        std::iter::once(
            xilem::window(
                state.window,
                "Recite — Component specimen",
                view::specimen(state),
            )
            .with_default_properties(state.theme.properties.clone())
            .with_base_color(state.theme.palette.canvas)
            .with_options(|_| {
                xilem::WindowOptions::new("Recite — Component specimen")
                    .with_initial_inner_size(dpi::LogicalSize::new(1200., 800.))
                    .with_min_inner_size(dpi::LogicalSize::new(760., 650.))
                    .on_close(|state: &mut Specimen| state.closed = true)
            }),
        )
    })
    .with_default_properties(application_defaults)
    .with_font(include_bytes!("../../freya/fonts/literata/Literata-Variable.ttf").to_vec())
    .with_font(include_bytes!("../../freya/fonts/ibm-plex-mono/IBMPlexMono-Regular.ttf").to_vec());
    let event_loop = xilem::EventLoop::with_user_event()
        .build()
        .map_err(|error| error.to_string())?;
    let proxy = event_loop.create_proxy();
    let (driver, windows) = application
        .into_driver_and_windows(move |event| proxy.send_event(event).map_err(|error| error.0));
    crate::native_host::run(event_loop, windows, driver, defaults)
        .map_err(|error| error.to_string())
}
