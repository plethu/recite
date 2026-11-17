//! Personal presentation is a cached projection of the shared authored palette.
mod emphasized_text;
mod excerpt;
mod fit_content;
pub(crate) use excerpt::excerpt;
pub(crate) use fit_content::fit_content;
mod overhang;
pub(crate) use overhang::{hang_below, overhang};
mod reveal;
pub(crate) use reveal::revealed;
pub(crate) mod banner;
mod controls;
mod disclosure;
pub(crate) use disclosure::{chooser, disclosure, disclosure_width, route_name, route_toggle};
mod styled_text;
pub(crate) use emphasized_text::search_label;
pub(crate) use styled_text::{manuscript_text, source_text};
pub(crate) mod control_style;
pub(crate) mod diff;
pub(crate) mod double_click;
pub(crate) mod grid;
pub(crate) mod group;
mod icons;
pub(crate) mod identity;
pub(crate) mod material;
pub(crate) mod print;
pub(crate) use icons::{FACE, Icon, SMALL, mark};
pub(crate) mod marks;
pub(crate) mod number;
mod radio;
pub(crate) mod row;
pub(crate) mod running;
mod search;
pub(crate) mod segments;
pub(crate) mod slip;
pub(crate) use search::search_field;
mod shaped_text;
pub(crate) mod sheet_head;
pub(crate) mod sheet_text;
pub(crate) use shaped_text::ShapedText;
mod theme;
pub(crate) mod tokens;
pub(super) use controls::{label, pane, paragraph, prose, text_button};
pub(crate) use radio::GroupStop;
pub(super) use radio::radio_button;
use recite_config::{WriterConfig, WriterPresentationField, WriterTheme};
use recite_writer_session::palette::Palette;
use std::sync::Arc;
pub(crate) use tokens::{Page, Sheet, Type, Typeset, control_inset, label_inset, space};
use xilem::{
    masonry::core::DefaultProperties,
    peniko::{Color, ImageBrush},
};

pub(super) struct Theme {
    key: (WriterTheme, bool, u16),
    pub palette: Palette<Color>,
    pub properties: Arc<DefaultProperties>,
    /// The desk's grain in this palette's grain ink.
    pub grain: Option<ImageBrush>,
    /// The modal backdrop's print texture in this palette's screen ink.
    pub screen: ImageBrush,
    /// The backdrop's texture: the dot screen, or the gallery's hatch comparison, which theme
    /// refreshes keep.
    pub texture: material::Texture,
    wordmark: Result<Arc<usvg::Tree>, String>,
    hero: Result<Arc<usvg::Tree>, String>,
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
            hero: identity::load_hero(config),
            grain: material::grain(palette.grain, config.theme == WriterTheme::Dark),
            screen: material::screen(material::Texture::default(), palette.screen),
            texture: material::Texture::default(),
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
            let texture = self.texture;
            *self = Self::new(config);
            self.texture = texture;
            self.screen = material::screen(texture, self.palette.screen);
        }
    }
}
pub(super) use recite_writer_app::presentation::{palette, scale};
