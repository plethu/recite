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
pub const EXCERPT_LINE_HEIGHT: f32 = 1.35;
/// A map card fits a title, speaker and three excerpt lines at `EXCERPT_LINE_HEIGHT`.
pub const CARD_HEIGHT: f32 = 160.;
pub fn code_size() -> f32 {
    f32::from(
        crate::presentation::typography().value(recite_config::WriterPresentationField::SourceSize),
    )
}
pub const CODE_LINE_HEIGHT: f32 = 1.4;
pub fn title() -> f32 {
    22. * ui_scale()
}
/// The beat title over the manuscript, in the display face.
pub fn display_title() -> f32 {
    32. * ui_scale()
}
/// The gap under each Script line that holds its hover meta and actions.
pub const META_GAP: f32 = 24.;
/// The Script margin that carries run-in speaker and section labels.
pub fn script_margin() -> f32 {
    96. * ui_scale()
}
/// Shorten a stable ID for meta text: the first and last four characters.
pub fn short_id(id: &str) -> String {
    let chars: Vec<char> = id.chars().collect();
    if chars.len() <= 10 {
        return id.to_owned();
    }
    let head: String = chars[..4].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{head}…{tail}")
}
pub const SPLITTER_WIDTH: f32 = 6.;
pub fn collapsed_drawer_width() -> f32 {
    control_height() + 8.
}
pub const RESIZE_STEP: f32 = 16.;
pub fn control_height() -> f32 {
    32. * ui_scale()
}
/// Controls are stamps with square-ish corners, not pills.
pub const RADIUS: f32 = 3.;
pub const DIALOG_RADIUS: f32 = 4.;
/// Stamp controls: an ink stroke, and how far hover and press lift a filled stamp.
pub const STROKE_WIDTH: f32 = 1.5;
pub const STAMP_HOVER: f32 = 0.12;
pub const STAMP_PRESSED: f32 = 0.22;
/// The sidebar's horizontal padding; selected rows' plates bleed across it.
pub const RAIL_PADDING: f32 = 12.;
/// A list row's reserved marker gutter, beyond its usual padding.
pub const ROW_GUTTER: f32 = 12.;
/// The thumb-index diamond beside a selected row, before its 45° turn.
pub const MARKER_SIZE: f32 = 7.;
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

/// Font fallbacks stay local; no network fonts are loaded. Writer bundles no font files, so each
/// chain names the intended OFL face first and falls back to the previous stack.
pub const FONT_UI: &[&str] = &[
    "IBM Plex Sans",
    "Inter",
    "Geist",
    "Segoe UI",
    "Helvetica Neue",
    "Noto Sans",
    // Scripts the bundled faces lack: macOS, then Linux (Noto) families, before the generic.
    // Skia also falls back per glyph through the system font manager; naming these fixes the
    // choice. One Han face serves Chinese and Japanese until text carries a locale.
    "PingFang SC",
    "Hiragino Sans",
    "Apple SD Gothic Neo",
    "Geeza Pro",
    "Kohinoor Devanagari",
    "Thonburi",
    "Noto Sans CJK SC",
    "Noto Sans CJK JP",
    "Noto Sans CJK KR",
    "Noto Sans Arabic",
    "Noto Sans Hebrew",
    "Noto Sans Devanagari",
    "Noto Sans Thai",
    "sans-serif",
];
pub const FONT_PROSE: &[&str] = &[
    "Literata",
    "Source Serif 4",
    "Noto Serif",
    "Georgia",
    // Serif companions where the system has them; the sans families cover the rest.
    "Songti SC",
    "Hiragino Mincho ProN",
    "Noto Serif CJK SC",
    "Noto Serif CJK JP",
    "Noto Serif CJK KR",
    "Noto Naskh Arabic",
    "Noto Serif Hebrew",
    "Noto Serif Devanagari",
    "Noto Serif Thai",
    // Scripts the bundled faces lack: macOS, then Linux (Noto) families, before the generic.
    // Skia also falls back per glyph through the system font manager; naming these fixes the
    // choice. One Han face serves Chinese and Japanese until text carries a locale.
    "PingFang SC",
    "Hiragino Sans",
    "Apple SD Gothic Neo",
    "Geeza Pro",
    "Kohinoor Devanagari",
    "Thonburi",
    "Noto Sans CJK SC",
    "Noto Sans CJK JP",
    "Noto Sans CJK KR",
    "Noto Sans Arabic",
    "Noto Sans Hebrew",
    "Noto Sans Devanagari",
    "Noto Sans Thai",
    "serif",
];
/// Pane titles and identity lockups; falls back to the reading face.
pub const FONT_DISPLAY: &[&str] = &[
    // The static 72pt Soft cut carries the print shape; the variable font defaults to opsz 144.
    "Fraunces 72pt Soft",
    "Fraunces",
    "Literata",
    "Source Serif 4",
    "Noto Serif",
    "Georgia",
    // Serif companions where the system has them; the sans families cover the rest.
    "Songti SC",
    "Hiragino Mincho ProN",
    "Noto Serif CJK SC",
    "Noto Serif CJK JP",
    "Noto Serif CJK KR",
    "Noto Naskh Arabic",
    "Noto Serif Hebrew",
    "Noto Serif Devanagari",
    "Noto Serif Thai",
    // Scripts the bundled faces lack: macOS, then Linux (Noto) families, before the generic.
    // Skia also falls back per glyph through the system font manager; naming these fixes the
    // choice. One Han face serves Chinese and Japanese until text carries a locale.
    "PingFang SC",
    "Hiragino Sans",
    "Apple SD Gothic Neo",
    "Geeza Pro",
    "Kohinoor Devanagari",
    "Thonburi",
    "Noto Sans CJK SC",
    "Noto Sans CJK JP",
    "Noto Sans CJK KR",
    "Noto Sans Arabic",
    "Noto Sans Hebrew",
    "Noto Sans Devanagari",
    "Noto Sans Thai",
    "serif",
];
/// Facts about the text rather than the text: file names, IDs, counts, shortcuts.
pub const FONT_MONO: &[&str] = &[
    "IBM Plex Mono",
    "Menlo",
    "Cascadia Mono",
    "DejaVu Sans Mono",
    // Scripts the bundled faces lack: macOS, then Linux (Noto) families, before the generic.
    // Skia also falls back per glyph through the system font manager; naming these fixes the
    // choice. One Han face serves Chinese and Japanese until text carries a locale.
    "PingFang SC",
    "Hiragino Sans",
    "Apple SD Gothic Neo",
    "Geeza Pro",
    "Kohinoor Devanagari",
    "Thonburi",
    "Noto Sans CJK SC",
    "Noto Sans CJK JP",
    "Noto Sans CJK KR",
    "Noto Sans Arabic",
    "Noto Sans Hebrew",
    "Noto Sans Devanagari",
    "Noto Sans Thai",
    "monospace",
];
/// Width over height of `assets/identity/recite-wordmark.svg`'s tight viewBox.
pub const WORDMARK_ASPECT: f32 = 3.337;
/// Width over height of `recite-wordmark-hero.svg`, the transparent print wordmark.
pub const HERO_WORDMARK_ASPECT: f32 = 3.251;
pub const WELCOME_WORDMARK_WIDTH: f32 = 420.;
pub const CARD_RADIUS: f32 = 4.;
/// Hard offset shadows: a solid plate, no blur. Cards, menus and pickers use the small offset.
pub const SHADOW_OFFSET: f32 = 4.;
/// The empty-state tint: teal dots fading in from the bottom-right corner.
pub const HALFTONE_FIELD_ALPHA: f32 = 0.28;
/// The selected card's dot-screen title strip.
pub const HALFTONE_STRIP_ALPHA: f32 = 0.35;
pub const DIALOG_SHADOW_OFFSET: f32 = 6.;
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

/// Tracked small capitals for structural labels such as speakers and section names.
pub fn caps(text: &str) -> Label {
    label()
        .text(text.to_uppercase())
        .a11y_alt(text.to_owned())
        .font_size(11. * ui_scale())
        .font_weight(FontWeight::SEMI_BOLD)
        .letter_spacing(1.32 * ui_scale())
        .color(colors().muted)
}
/// Facts about the text in the meta face: file names, IDs, counts and shortcuts.
pub fn meta(text: impl Into<String>) -> Label {
    label()
        .text(text.into())
        .meta_font()
        .font_size(12. * ui_scale())
        .color(colors().muted)
}

/// One run of a slug line; flags such as `default` are destinations in text-safe teal.
pub enum Slug {
    Meta(String),
    Flag(String),
}
/// The mono slug line under a pane title: file, block, flags and counts.
pub fn slug(parts: impl IntoIterator<Item = Slug>) -> Rect {
    let p = colors();
    let line = parts.into_iter().enumerate().fold(
        paragraph()
            .width(Size::fill())
            .meta_font()
            .font_size(12. * ui_scale())
            .color(p.muted),
        |row, (index, part)| {
            let row = if index == 0 {
                row
            } else {
                row.span(Span::new("  "))
            };
            row.span(match part {
                Slug::Meta(text) => Span::new(text),
                Slug::Flag(text) => Span::new(text).color(p.teal_text),
            })
        },
    );
    rect().width(Size::fill()).child(line)
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

/// Display and meta faces share the prose trait's explicit-chain approach.
pub trait PrintTypography: TextStyleExt {
    fn display_font(mut self) -> Self {
        self.get_text_style_data().font_families =
            FONT_DISPLAY.iter().map(|family| (*family).into()).collect();
        self.font_weight(FontWeight::MEDIUM)
    }
    fn meta_font(mut self) -> Self {
        self.get_text_style_data().font_families =
            FONT_MONO.iter().map(|family| (*family).into()).collect();
        self
    }
}
impl<T: TextStyleExt> PrintTypography for T {}

pub fn ui_scale() -> f32 {
    f32::from(
        crate::presentation::typography().value(recite_config::WriterPresentationField::UiScale),
    ) / 100.
}
