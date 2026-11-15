//! Margin marks, the same in the galley margin, the scene list and on map cards. They differ in
//! shape as well as ink: a solid teal bar for the current line or beat, a plum halftone bar for
//! unsaved work ("halftone means not yet fixed"), and a focusable error wedge for a problem.
#[cfg(test)]
mod tests;
mod wedge;
use super::material::{Bar, Decorated, Edge, bar};
use recite_writer_session::palette::Palette;
pub(crate) use wedge::{wedge, wedge_glyph};
use xilem::peniko::Color;

/// A margin bar's thickness.
pub(crate) const THICKNESS: f64 = 3.;
/// A decorative wedge's side, where a row or card repeats a problem its name already gives.
pub(crate) const GLYPH: f64 = 16.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mark {
    /// The current line or beat.
    Current,
    /// Work not yet saved.
    Unsaved,
}
impl Mark {
    fn bar(self, palette: &Palette<Color>) -> Bar {
        Bar {
            thickness: THICKNESS,
            screen: self == Self::Unsaved,
            ..Bar::solid(
                match self {
                    Self::Current => palette.teal,
                    Self::Unsaved => palette.accent,
                },
                Edge::Leading,
                0.,
                0.,
            )
        }
    }
}

/// `child` with `mark` down its leading edge, or unmarked.
pub(crate) fn marked<V>(palette: &Palette<Color>, mark: Option<Mark>, child: V) -> Decorated<V> {
    bar(mark.map(|mark| mark.bar(palette)), child)
}
