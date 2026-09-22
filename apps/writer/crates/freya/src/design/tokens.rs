//! Logical pixels; graph world geometry is deliberately owned by the map.
use freya::{
    animation::{AnimNum, Ease, Function},
    prelude::*,
};

pub const SPACE_XS: f32 = 4.;
pub const SPACE_SM: f32 = 8.;
pub const SPACE_MD: f32 = 12.;
pub const SPACE_LG: f32 = 16.;
pub const SPACE_XL: f32 = 24.;
pub const PANEL_PADDING: f32 = 20.;
pub fn small() -> f32 {
    13. * ui_scale()
}
pub fn body() -> f32 {
    14. * ui_scale()
}
pub fn heading() -> f32 {
    18. * ui_scale()
}
pub fn prose_size() -> f32 {
    f32::from(
        crate::presentation::typography()
            .value(recite_config::WriterPresentationField::ReadingSize),
    )
}
pub const TEXT_EXCERPT: f32 = 15.;
pub const EXCERPT_LINE_HEIGHT: f32 = 1.28;
pub fn code_size() -> f32 {
    f32::from(
        crate::presentation::typography().value(recite_config::WriterPresentationField::SourceSize),
    )
}
pub const CODE_LINE_HEIGHT: f32 = 1.4;
pub fn title() -> f32 {
    22. * ui_scale()
}
pub const SPLITTER_WIDTH: f32 = 6.;
pub fn collapsed_drawer_width() -> f32 {
    control_height() + 8.
}
pub const RESIZE_STEP: f32 = 16.;
pub fn control_height() -> f32 {
    32. * ui_scale()
}
pub const RADIUS: f32 = 6.;
pub const DIALOG_RADIUS: f32 = 10.;
pub const FOCUS_WIDTH: f32 = 2.;
pub const MOTION_MS: u64 = 160;
pub const HOVER_MS: u64 = 100;
pub const SELECTION_MS: u64 = 200;

pub fn transition(from: f32, to: f32, reduced: bool) -> AnimNum {
    AnimNum::new(from, to)
        .time(if reduced { 0 } else { MOTION_MS })
        .ease(Ease::Out)
        .function(Function::Cubic)
}

pub fn colors() -> super::palette::Palette {
    super::palette::current()
}

/// Font fallbacks stay local; no network fonts are loaded.
pub const FONT_UI: &[&str] = &[
    "Inter",
    "Geist",
    "Segoe UI",
    "Helvetica Neue",
    "Noto Sans",
    "sans-serif",
];
pub const FONT_PROSE: &[&str] = &[
    "Literata",
    "Source Serif 4",
    "Noto Serif",
    "Georgia",
    "serif",
];
pub const CARD_RADIUS: f32 = 8.;
pub const ICON_SIZE: f32 = 18.;
pub fn picker_row_height() -> f32 {
    44. * ui_scale()
}
pub const PICKER_MAX_WIDTH: f32 = 400.;

/// Two short, exclusive setting labels.
pub const OPTION_GROUP_WIDTH: f32 = 200.;

/// Reserve space for passage actions so focusing prose never shifts its text.
pub fn prose_meta_height() -> f32 {
    control_height()
}

/// UI and prose frames share explicit fallback chains across every renderer.
pub fn interface() -> Rect {
    FONT_UI
        .iter()
        .fold(rect().font_size(body()), |frame, family| {
            frame.font_family(*family)
        })
}
pub fn prose() -> Rect {
    rect().prose_font().font_size(prose_size())
}

/// The same reading face applies to editable paragraphs, previews, and graph excerpts.
pub trait ProseTypography: TextStyleExt {
    fn prose_font(mut self) -> Self {
        self.get_text_style_data().font_families =
            FONT_PROSE.iter().map(|family| (*family).into()).collect();
        self.font_weight(FontWeight::NORMAL)
    }
}
impl<T: TextStyleExt> ProseTypography for T {}

pub fn ui_scale() -> f32 {
    f32::from(
        crate::presentation::typography().value(recite_config::WriterPresentationField::UiScale),
    ) / 100.
}
