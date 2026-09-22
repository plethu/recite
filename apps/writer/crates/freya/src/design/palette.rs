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
    pub shadow: Color,
    pub error: Color,
}
impl Palette {
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
            selection: color(dark, (220, 232, 220), (51, 72, 58)),
            hover: color(dark, (235, 236, 232), (56, 61, 62)),
            pressed: color(dark, (218, 221, 215), (72, 79, 79)),
            shadow: Color::from_argb(if dark { 85 } else { 28 }, 0, 0, 0),
            error: color(dark, (150, 63, 54), (242, 179, 166)),
        }
    }
}

pub fn current() -> Palette {
    let theme = use_theme();
    let theme = theme.read();
    theme
        .get::<Palette>("writer_palette")
        .copied()
        .unwrap_or_else(|| Palette::new(false))
}

pub fn syntax(dark: bool) -> EditorSyntaxTheme {
    let mut theme = if dark {
        EditorSyntaxTheme::dark()
    } else {
        EditorSyntaxTheme::light()
    };
    theme.text = text(dark);
    theme.keyword = color(dark, (54, 95, 146), (166, 200, 240));
    theme.punctuation_special = theme.keyword;
    theme.label = color(dark, (114, 82, 139), (210, 181, 236));
    theme.variable = theme.label;
    theme.function = color(dark, (38, 105, 103), (138, 207, 198));
    theme.function_method = theme.function;
    theme.constant = color(dark, (134, 85, 32), (230, 190, 133));
    theme.string = theme.constant;
    theme.string_special = theme.text;
    theme.number = theme.constant;
    theme.comment = color(dark, (101, 97, 91), (188, 184, 177));
    theme.property = theme.comment;
    theme
}

fn color(dark: bool, light: (u8, u8, u8), night: (u8, u8, u8)) -> Color {
    let (r, g, b) = if dark { night } else { light };
    Color::from_rgb(r, g, b)
}

pub fn reading(dark: bool) -> Color {
    Palette::new(dark).surface
}
pub fn muted(dark: bool) -> Color {
    Palette::new(dark).muted
}
pub fn rule(dark: bool) -> Color {
    Palette::new(dark).rule
}
pub fn accent(dark: bool) -> Color {
    Palette::new(dark).accent
}
pub fn text(dark: bool) -> Color {
    Palette::new(dark).ink
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
