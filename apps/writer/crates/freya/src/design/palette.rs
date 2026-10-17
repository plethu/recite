//! Semantic colour ownership for both Freya widgets and custom map painting.
use freya::{code_editor::*, prelude::*};
mod theme;
pub use theme::{theme, theme_with};

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
    /// Riso second ink: plates, rules and decoration only, never text in light mode.
    pub teal: Color,
    /// Text-safe second ink, for links and destinations only.
    pub teal_text: Color,
    /// Second-ink selection plate; carries ink or accent text, not `teal_text` when pressed.
    pub plate: Color,
    /// Wordmark letters: plum on paper, warm paper on dark surfaces.
    pub wordmark: Color,
    /// Filled ink for the primary action and the active segment; text on it is `on_stamp`.
    pub stamp: Color,
    pub on_stamp: Color,
    /// Plum multiplied by teal: the darkest ink, used where two inks meet.
    pub overprint: Color,
    /// Overprint navy in light mode; on dark paper it would not separate, so dark uses `floating`.
    pub status_bar: Color,
    /// A hairline above the dark status bar; matches the bar in light mode.
    pub status_edge: Color,
    pub status_text: Color,
    /// Mint on the status bar in both modes, for a clean diagnostics state.
    pub status_signal: Color,
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
                &mut p.wordmark,
                &mut p.stamp,
                &mut p.on_stamp,
                &mut p.teal,
                &mut p.teal_text,
                &mut p.plate,
                &mut p.overprint,
                &mut p.status_bar,
                &mut p.status_edge,
                &mut p.status_text,
                &mut p.status_signal,
            ] {
                *c = gray(*c);
            }
        }
        p
    }
    pub fn new(dark: bool) -> Self {
        let surface = color(dark, (250, 246, 239), (42, 38, 46));
        let ink = color(dark, (48, 36, 49), (243, 233, 223));
        // Mint replaces riso teal on dark paper, where it is also text-safe.
        let second_ink = color(dark, (0, 131, 138), (130, 216, 213));
        let floating = color(dark, (255, 251, 245), (54, 48, 57));
        let muted = color(dark, (83, 69, 81), (207, 192, 203));
        let rule = Color::lerp(surface, ink, 0.14);
        let overprint = Color::from_rgb(0, 32, 53);
        Self {
            canvas: color(dark, (239, 233, 225), (27, 24, 30)),
            surface,
            inset: color(dark, (243, 237, 229), (29, 26, 33)),
            floating,
            // Hairlines are ink at 14% over the reading surface, precomputed so map painting
            // and other opaque consumers draw the same colour.
            rule,
            boundary: color(dark, (127, 116, 119), (153, 139, 154)),
            ink,
            muted,
            placeholder: color(dark, (113, 98, 109), (175, 157, 172)),
            accent: color(dark, (104, 62, 98), (212, 174, 204)),
            on_accent: color(dark, (255, 251, 245), (40, 25, 39)),
            selection: color(dark, (234, 221, 232), (72, 54, 72)),
            hover: color(dark, (238, 229, 231), (58, 51, 61)),
            pressed: color(dark, (218, 203, 215), (82, 66, 82)),
            backdrop: Color::from_argb(110, 0, 0, 0),
            // Soft blurred shadows are gone; elevation is a hard offset plate (`material::offset`).
            shadow: Color::TRANSPARENT,
            error: color(dark, (137, 51, 48), (246, 191, 178)),
            wordmark: if dark {
                ink
            } else {
                Color::from_rgb(104, 62, 98)
            },
            stamp: ink,
            on_stamp: surface,
            teal: second_ink,
            teal_text: color(dark, (15, 107, 111), (130, 216, 213)),
            // 12% second ink on paper; on dark paper, 14% mint over the surface.
            plate: color(dark, (214, 221, 213), (54, 63, 69)),
            overprint,
            status_bar: if dark { floating } else { overprint },
            status_edge: if dark { rule } else { overprint },
            status_text: if dark {
                muted
            } else {
                Color::from_rgb(243, 233, 223)
            },
            status_signal: Color::from_rgb(130, 216, 213),
        }
    }

    /// Reply destinations use text-safe teal, except on a pressed fill where it fails contrast.
    pub fn destination(&self, pressed: bool) -> Color {
        if pressed { self.ink } else { self.teal_text }
    }

    /// Hover and press lift a filled stamp toward its text colour, keeping text readable.
    pub fn stamp_state(&self, amount: f32) -> Color {
        Color::lerp(self.stamp, self.on_stamp, amount)
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

/// Project palette RGB into the graph renderer; edge opacity is applied separately.
pub(crate) fn skia_opaque(color: Color) -> skia_safe::Color {
    skia_safe::Color::from_rgb(color.r(), color.g(), color.b())
}

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
