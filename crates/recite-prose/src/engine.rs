//! The boundary between Recite prose regions and a checking engine.

use std::ops::Range;

use recite_parser::ProseText;

/// The class of a finding, which selects its stable diagnostic code.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
#[non_exhaustive]
pub enum ProseFindingKind {
    /// `RECITE_PROSE001`: a word missing from the dictionaries.
    Spelling,
    /// `RECITE_PROSE002`: grammar, usage, punctuation and typos.
    Grammar,
    /// `RECITE_PROSE003`: style and readability suggestions.
    Style,
}

/// One engine finding in checked-text byte coordinates.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EngineFinding {
    pub kind: ProseFindingKind,
    /// The engine's stable rule name, used for suppression.
    pub rule: String,
    /// The engine's English explanation.
    pub message: String,
    pub range: Range<usize>,
    /// Replacements, each for a checked-text byte range.
    pub suggestions: Vec<(Range<usize>, String)>,
}

/// A spelling and grammar engine. Engines receive masked [`ProseText`] and
/// never see Recite syntax other than placeholders, which they must treat as
/// opaque. Implementations must be deterministic for the same input.
pub trait ProseEngine: Send {
    fn check(&mut self, text: &ProseText) -> Vec<EngineFinding>;
}
