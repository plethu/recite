//! Personal presentation is a cached projection of the shared authored palette.
mod controls;
mod radio;
mod theme;
pub(super) use controls::{label, paragraph, prose, text_button};
pub(super) use radio::radio_button;
use recite_config::{WriterConfig, WriterPresentationField, WriterTheme};
use recite_writer_session::palette::{Palette, Rgba};
use std::sync::Arc;
use xilem::{masonry::core::DefaultProperties, peniko::Color};

pub(super) struct Theme {
    key: (WriterTheme, bool, u16),
    pub palette: Palette<Color>,
    pub properties: Arc<DefaultProperties>,
}
impl Theme {
    pub fn initial_properties(&self) -> DefaultProperties {
        theme::properties(&self.palette, f32::from(self.key.2) / 100.)
    }
    pub fn new(config: &WriterConfig) -> Self {
        let palette = palette(config);
        Self {
            key: (
                config.theme,
                config.monochrome,
                config.presentation.value(WriterPresentationField::UiScale),
            ),
            properties: Arc::new(theme::properties(&palette, scale(config))),
            palette,
        }
    }
    pub fn refresh(&mut self, config: &WriterConfig) {
        let key = (
            config.theme,
            config.monochrome,
            config.presentation.value(WriterPresentationField::UiScale),
        );
        if self.key != key {
            *self = Self::new(config);
        }
    }
}
pub(super) fn scale(config: &WriterConfig) -> f32 {
    f32::from(config.presentation.value(WriterPresentationField::UiScale)) / 100.
}
pub(super) fn palette(config: &WriterConfig) -> Palette<Color> {
    Palette::with_mode(config.theme == WriterTheme::Dark, config.monochrome).map(convert)
}
fn convert(color: Rgba) -> Color {
    Color::from_rgba8(color.r, color.g, color.b, color.a)
}
