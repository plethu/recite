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
            canvas: color(dark, (240, 240, 237), (29, 31, 33)),
            surface: color(dark, (255, 254, 251), (38, 41, 43)),
            inset: color(dark, (244, 244, 240), (30, 33, 35)),
            floating: color(dark, (255, 254, 252), (48, 52, 54)),
            rule: color(dark, (217, 219, 212), (64, 69, 71)),
            boundary: color(dark, (129, 125, 116), (133, 142, 141)),
            ink: color(dark, (47, 48, 44), (234, 233, 226)),
            muted: color(dark, (79, 82, 77), (198, 203, 197)),
            placeholder: color(dark, (108, 110, 105), (153, 160, 155)),
            accent: color(dark, (60, 89, 68), (177, 205, 179)),
            on_accent: color(dark, (255, 254, 252), (27, 40, 30)),
            selection: color(dark, (231, 233, 229), (57, 63, 60)),
            hover: color(dark, (235, 236, 232), (56, 61, 62)),
            pressed: color(dark, (213, 215, 212), (72, 76, 75)),
            backdrop: Color::from_argb(110, 0, 0, 0),
            shadow: Color::from_argb(if dark { 85 } else { 28 }, 0, 0, 0),
            error: color(dark, (150, 63, 54), (246, 191, 178)),
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
