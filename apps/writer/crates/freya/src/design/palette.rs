//! Semantic colour ownership for both Freya widgets and custom map painting.
use freya::{code_editor::*, prelude::*};
mod theme;
pub use theme::theme;

#[derive(Clone, Copy)]
pub(crate) struct Palette {
    pub canvas: Color,
    pub surface: Color,
    pub inset: Color,
    pub floating: Color,
    pub rule: Color,
    pub boundary: Color,
    pub ink: Color,
    pub muted: Color,
    pub placeholder: Color,
    pub accent: Color,
    pub on_accent: Color,
    pub selection: Color,
    pub hover: Color,
    pub pressed: Color,
    pub backdrop: Color,
    pub shadow: Color,
    pub error: Color,
}
impl Palette {
    pub fn with_mode(dark: bool, monochrome: bool) -> Self {
        let mut p = Self::new(dark);
        if monochrome {
            for c in [
                &mut p.canvas,
                &mut p.surface,
                &mut p.inset,
                &mut p.floating,
                &mut p.rule,
                &mut p.boundary,
                &mut p.ink,
                &mut p.muted,
                &mut p.placeholder,
                &mut p.accent,
                &mut p.on_accent,
                &mut p.selection,
                &mut p.hover,
                &mut p.pressed,
                &mut p.error,
            ] {
                *c = gray(*c);
            }
        }
        p
    }
    pub fn new(dark: bool) -> Self {
        Self {
            canvas: color(dark, (239, 233, 225), (27, 24, 30)),
            surface: color(dark, (250, 246, 239), (36, 32, 39)),
            inset: color(dark, (243, 237, 229), (29, 26, 33)),
            floating: color(dark, (255, 251, 245), (46, 41, 49)),
            rule: color(dark, (216, 206, 201), (65, 57, 68)),
            boundary: color(dark, (127, 116, 119), (153, 139, 154)),
            ink: color(dark, (48, 36, 49), (243, 233, 223)),
            muted: color(dark, (83, 69, 81), (207, 192, 203)),
            placeholder: color(dark, (113, 98, 109), (175, 157, 172)),
            accent: color(dark, (104, 62, 98), (212, 174, 204)),
            on_accent: color(dark, (255, 251, 245), (40, 25, 39)),
            selection: color(dark, (234, 221, 232), (59, 44, 59)),
            hover: color(dark, (238, 229, 231), (54, 46, 56)),
            pressed: color(dark, (218, 203, 215), (69, 55, 69)),
            backdrop: Color::from_argb(110, 0, 0, 0),
            shadow: Color::from_argb(if dark { 85 } else { 28 }, 0, 0, 0),
            error: color(dark, (137, 51, 48), (246, 191, 178)),
        }
    }
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

pub fn display_name(identifier: &str) -> String {
    let name = identifier
        .strip_suffix(".recite")
        .unwrap_or(identifier)
        .replace('_', " ");
    name.split(' ')
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().collect::<String>() + chars.as_str()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests;

fn monochrome() -> bool {
    try_consume_context::<crate::presentation::Typography>()
        .is_some_and(|p| p.0.read().config.writer.monochrome)
}
fn gray(color: Color) -> Color {
    let linear = |c: u8| {
        let v = f64::from(c) / 255.;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let l = 0.2126 * linear(color.r()) + 0.7152 * linear(color.g()) + 0.0722 * linear(color.b());
    let v = if l <= 0.0031308 {
        l * 12.92
    } else {
        1.055 * l.powf(1. / 2.4) - 0.055
    };
    let c = (v * 255.).round() as u8;
    Color::from_argb(color.a(), c, c, c)
}
