//! Personal presentation is a cached projection of the shared authored palette.
mod emphasized_text;
mod excerpt;
mod fit_content;
pub(crate) use excerpt::excerpt;
pub(crate) use fit_content::fit_content;
mod controls;
mod styled_text;
pub(crate) use emphasized_text::{
    manuscript_comparison, manuscript_search, search_label, source_comparison,
};
pub(crate) use styled_text::{manuscript_text, source_text};
pub(crate) mod control_style;
pub(crate) mod group;
mod icons;
pub(crate) mod identity;
pub(crate) mod material;
pub(crate) mod print;
pub(crate) use icons::Icon;
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
    wordmark: Result<Arc<usvg::Tree>, String>,
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
            wordmark: identity::load(config),
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
