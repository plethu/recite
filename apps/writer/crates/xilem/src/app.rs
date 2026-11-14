//! Native window ownership over the toolkit-independent application shell.
#[cfg(target_os = "linux")]
mod activation;
pub(super) mod keyboard;
mod opening;
pub(super) mod picker;
mod settings;
mod slot;
mod view;
mod welcome;
pub(crate) use recite_writer_app::shell::*;
use recite_writer_session::{startup::LaunchOptions, typography::BUNDLED_FONTS};
use std::ops::{Deref, DerefMut};
use xilem::winit::dpi;
use xilem::winit::error;
use xilem::{AppState, WindowId};

pub(crate) use welcome::ProjectField;

/// The shell plus what only this native window owns: its identity, theme and measurements.
pub(super) struct App {
    shell: Shell,
    window: WindowId,
    theme: crate::presentation::Theme,
    welcome_layout: welcome::Measurements,
}
impl Deref for App {
    type Target = Shell;
    fn deref(&self) -> &Shell {
        &self.shell
    }
}
impl DerefMut for App {
    fn deref_mut(&mut self) -> &mut Shell {
        &mut self.shell
    }
}
pub(super) fn run(app: App) -> Result<(), error::EventLoopError> {
    let defaults = app.theme.initial_properties();
    let event_loop = xilem::EventLoop::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    let (driver, windows) = application(app)
        .into_driver_and_windows(move |event| proxy.send_event(event).map_err(|error| error.0));
    crate::native_host::run(event_loop, windows, driver, defaults)
}
impl AppState for App {
    fn keep_running(&self) -> bool {
        self.shell.keep_running()
    }
}
impl App {
    pub fn new(options: LaunchOptions) -> Result<Option<Self>, String> {
        Ok(Shell::new(options)?.map(Self::wrap))
    }
    fn wrap(mut shell: Shell) -> Self {
        shell.host = crate::native_host::services();
        let mut theme = crate::presentation::Theme::new(&recite_config::WriterConfig::default());
        theme.refresh(&shell.preferences.config.writer);
        Self {
            shell,
            window: WindowId::next(),
            theme,
            welcome_layout: Default::default(),
        }
    }
}
pub(super) fn application(
    app: App,
) -> xilem::Xilem<App, impl FnMut(&mut App) -> std::iter::Once<xilem::WindowView<App>>> {
    let defaults = app.theme.initial_properties();
    let application = xilem::Xilem::new(app, |app: &mut App| {
        // Preferences may change between rebuilds; the theme cache follows them here.
        app.theme.refresh(&app.shell.preferences.config.writer);
        std::iter::once(
            xilem::window(app.window, "Recite Writer", view::app(app))
                .with_default_properties(app.theme.properties.clone())
                .with_base_color(app.theme.palette.canvas)
                .with_options(|_| {
                    xilem::WindowOptions::new("Recite Writer")
                        .with_initial_inner_size(dpi::LogicalSize::new(1200., 800.))
                        .with_min_inner_size(dpi::LogicalSize::new(900., 650.))
                        .on_close(|app: &mut App| app.request_close())
                }),
        )
    })
    .with_default_properties(defaults);
    BUNDLED_FONTS
        .iter()
        .fold(application, |app, (_, bytes)| app.with_font(bytes.to_vec()))
}

#[cfg(test)]
pub(crate) mod tests;
