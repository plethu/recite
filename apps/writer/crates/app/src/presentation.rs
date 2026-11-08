//! Personal presentation values derived from the Writer configuration.
use recite_config::{WriterConfig, WriterPresentationField, WriterTheme};
use recite_writer_session::palette::{Palette, Rgba};

/// The display colour every frontend renders with.
pub type Color = color::AlphaColor<color::Srgb>;

/// The interface scale factor chosen in personal settings.
pub fn scale(config: &WriterConfig) -> f32 {
    f32::from(config.presentation.value(WriterPresentationField::UiScale)) / 100.
}

/// The shared authored palette resolved for the chosen theme.
pub fn palette(config: &WriterConfig) -> Palette<Color> {
    Palette::with_mode(config.theme == WriterTheme::Dark, config.monochrome).map(convert)
}

fn convert(color: Rgba) -> Color {
    Color::from_rgba8(color.r, color.g, color.b, color.a)
}
