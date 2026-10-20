//! Semantic colour ownership for both Freya widgets and custom map painting.
use freya::{code_editor::*, prelude::*};
mod theme;
pub use theme::{theme, theme_with};

#[derive(Clone, Copy)]
pub(crate) struct Palette {
    source: recite_writer_session::palette::Palette,
    native: recite_writer_session::palette::Palette<Color>,
}
impl std::ops::Deref for Palette {
    type Target = recite_writer_session::palette::Palette<Color>;
    fn deref(&self) -> &Self::Target {
        &self.native
    }
}
impl Palette {
    pub fn new(dark: bool) -> Self {
        Self::with_mode(dark, false)
    }
    pub fn with_mode(dark: bool, monochrome: bool) -> Self {
        let source = recite_writer_session::palette::Palette::with_mode(dark, monochrome);
        Self {
            source,
            native: source.map(convert),
        }
    }
    pub fn destination(&self, pressed: bool) -> Color {
        convert(self.source.destination(pressed))
    }
    pub fn stamp_state(&self, amount: f32) -> Color {
        convert(self.source.stamp_state(amount))
    }
}
fn convert(color: recite_writer_session::palette::Rgba) -> Color {
    Color::from_argb(color.a, color.r, color.g, color.b)
}

pub fn current() -> Palette {
    let theme = get_theme_or_default();
    let theme = theme.read();
    theme
        .get::<Palette>("writer_palette")
        .copied()
        .unwrap_or_else(|| Palette::new(false))
}

pub fn syntax(dark: bool) -> EditorSyntaxTheme {
    syntax_with_mode(dark, monochrome())
}
fn syntax_with_mode(dark: bool, monochrome: bool) -> EditorSyntaxTheme {
    let p = Palette::with_mode(dark, monochrome);
    let token = |light, night| {
        let c = color(dark, light, night);
        if monochrome { gray(c) } else { c }
    };
    let keyword = token((54, 95, 146), (166, 200, 240));
    let name = token((114, 82, 139), (210, 181, 236));
    let function = token((38, 105, 103), (138, 207, 198));
    let constant = token((134, 85, 32), (230, 190, 133));
    // Exhaustive construction keeps new upstream token kinds in our colour review.
    EditorSyntaxTheme {
        text: p.ink,
        whitespace: p.muted,
        attribute: keyword,
        boolean: constant,
        comment: p.muted,
        constant,
        constructor: function,
        escape: constant,
        function,
        function_macro: function,
        function_method: function,
        keyword,
        label: name,
        module: name,
        number: constant,
        operator: p.ink,
        property: p.muted,
        punctuation: p.ink,
        punctuation_bracket: p.ink,
        punctuation_delimiter: p.ink,
        punctuation_special: keyword,
        string: constant,
        string_escape: constant,
        string_special: p.ink,
        tag: keyword,
        text_literal: p.ink,
        text_reference: name,
        text_title: keyword,
        text_uri: function,
        text_emphasis: p.ink,
        type_: name,
        variable: name,
        variable_builtin: name,
        variable_parameter: name,
    }
}

fn color(dark: bool, light: (u8, u8, u8), night: (u8, u8, u8)) -> Color {
    let (r, g, b) = if dark { night } else { light };
    Color::from_rgb(r, g, b)
}

pub fn reading(dark: bool) -> Color {
    Palette::with_mode(dark, monochrome()).surface
}
pub fn muted(dark: bool) -> Color {
    Palette::with_mode(dark, monochrome()).muted
}
pub fn rule(dark: bool) -> Color {
    Palette::with_mode(dark, monochrome()).rule
}
pub fn accent(dark: bool) -> Color {
    Palette::with_mode(dark, monochrome()).accent
}

pub use recite_writer_session::choices::display_name;

#[cfg(test)]
mod tests;

/// Project palette RGB into the graph renderer; edge opacity is applied separately.
pub(crate) fn skia_opaque(color: Color) -> skia_safe::Color {
    skia_safe::Color::from_rgb(color.r(), color.g(), color.b())
}

fn monochrome() -> bool {
    try_consume_context::<crate::presentation::Typography>()
        .is_some_and(|p| p.0.read().config.writer.monochrome)
}
fn gray(color: Color) -> Color {
    convert(
        recite_writer_session::palette::Rgba::from_argb(color.a(), color.r(), color.g(), color.b())
            .monochrome(),
    )
}
