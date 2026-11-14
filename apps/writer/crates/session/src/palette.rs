//! Authored Writer colours are independent of the native rendering toolkit.
mod rgba;
pub use rgba::Rgba;

pub const STAMP_HOVER: f32 = 0.12;
pub const STAMP_PRESSED: f32 = 0.22;
/// The grain tile's peak alpha; `assets/identity/grain.png` never exceeds it.
pub const GRAIN_PEAK: f32 = 0.07;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette<C = Rgba> {
    pub canvas: C,
    pub surface: C,
    pub inset: C,
    pub floating: C,
    pub rule: C,
    pub boundary: C,
    pub ink: C,
    pub muted: C,
    pub placeholder: C,
    pub accent: C,
    pub on_accent: C,
    pub selection: C,
    pub hover: C,
    pub pressed: C,
    pub backdrop: C,
    pub shadow: C,
    pub error: C,
    /// Riso second ink: plates, rules and decoration only, never text in light mode.
    pub teal: C,
    /// Text-safe second ink, for links and destinations only.
    pub teal_text: C,
    /// Second-ink selection plate; carries ink or accent text, not `teal_text` when pressed.
    pub plate: C,
    /// Wordmark letters: plum on paper, warm paper on dark surfaces.
    pub wordmark: C,
    /// Filled ink for the primary action and the active segment; text on it is `on_stamp`.
    pub stamp: C,
    pub on_stamp: C,
    /// Plum multiplied by teal: the darkest ink, used where two inks meet.
    pub overprint: C,
    /// Overprint navy in light mode; on dark paper it would not separate, so dark uses `floating`.
    pub status_bar: C,
    /// A hairline above the dark status bar; matches the bar in light mode.
    pub status_edge: C,
    pub status_text: C,
    /// Mint on the status bar in both modes, for a clean diagnostics state.
    pub status_signal: C,
    /// The hard shadow under the Script sheet: decorative, never a control boundary.
    pub stock: C,
    /// Paper-grain ink, painted at the tile's own alpha on desk surfaces only.
    pub grain: C,
}
impl Palette<Rgba> {
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
                &mut p.stock,
                &mut p.grain,
            ] {
                *c = c.monochrome();
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
        let rule = Rgba::mix(surface, ink, 0.14);
        let overprint = Rgba::from_rgb(0, 32, 53);
        let canvas = color(dark, (239, 233, 225), (27, 24, 30));
        Self {
            canvas,
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
            backdrop: Rgba::from_argb(110, 0, 0, 0),
            // Soft blurred shadows are gone; elevation is a hard offset plate (`material::offset`).
            shadow: Rgba::TRANSPARENT,
            error: color(dark, (137, 51, 48), (246, 191, 178)),
            wordmark: if dark {
                ink
            } else {
                Rgba::from_rgb(104, 62, 98)
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
                Rgba::from_rgb(243, 233, 223)
            },
            status_signal: Rgba::from_rgb(130, 216, 213),
            stock: Rgba::mix(canvas, ink, 0.20),
            // The identity print recipe's grain ink; the dark variant is a later pass.
            grain: Rgba::from_rgb(64, 46, 51),
        }
    }

    /// The darkest point of the grained desk, where text on the desk is checked.
    pub fn grained_desk(&self) -> Rgba {
        Rgba::mix(self.canvas, self.grain, GRAIN_PEAK)
    }

    /// Reply destinations use text-safe teal, except on a pressed fill where it fails contrast.
    pub fn destination(&self, pressed: bool) -> Rgba {
        if pressed { self.ink } else { self.teal_text }
    }

    /// Hover and press lift a filled stamp toward its text colour, keeping text readable.
    pub fn stamp_state(&self, amount: f32) -> Rgba {
        Rgba::mix(self.stamp, self.on_stamp, amount)
    }
}

impl<C> Palette<C> {
    pub fn map<T>(self, convert: impl Fn(C) -> T) -> Palette<T> {
        Palette {
            canvas: convert(self.canvas),
            surface: convert(self.surface),
            inset: convert(self.inset),
            floating: convert(self.floating),
            rule: convert(self.rule),
            boundary: convert(self.boundary),
            ink: convert(self.ink),
            muted: convert(self.muted),
            placeholder: convert(self.placeholder),
            accent: convert(self.accent),
            on_accent: convert(self.on_accent),
            selection: convert(self.selection),
            hover: convert(self.hover),
            pressed: convert(self.pressed),
            backdrop: convert(self.backdrop),
            shadow: convert(self.shadow),
            error: convert(self.error),
            teal: convert(self.teal),
            teal_text: convert(self.teal_text),
            plate: convert(self.plate),
            wordmark: convert(self.wordmark),
            stamp: convert(self.stamp),
            on_stamp: convert(self.on_stamp),
            overprint: convert(self.overprint),
            status_bar: convert(self.status_bar),
            status_edge: convert(self.status_edge),
            status_text: convert(self.status_text),
            status_signal: convert(self.status_signal),
            stock: convert(self.stock),
            grain: convert(self.grain),
        }
    }
}

fn color(dark: bool, light: (u8, u8, u8), night: (u8, u8, u8)) -> Rgba {
    let (r, g, b) = if dark { night } else { light };
    Rgba::from_rgb(r, g, b)
}

#[cfg(test)]
mod tests;
