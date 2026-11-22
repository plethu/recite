//! The document's problems: its own diagnostics, then any current prose findings.
use super::Checked;
use crate::Writer;
use recite_core::Diagnostic;
use recite_writer_model::DiagnosticSnapshot;
use recite_writer_session::prose::{ProseFinding, ProseFindingKind};

/// An indexed view over the accepted diagnostics followed by the current prose result.
pub struct Problems<'a> {
    document: DiagnosticSnapshot,
    prose: Option<&'a Checked>,
}

/// One listed problem; prose findings carry what their fixes need.
pub struct Problem<'a> {
    pub diagnostic: &'a Diagnostic,
    pub prose: Option<ProseProblem<'a>>,
}

/// A prose finding, addressed by the check that produced it.
pub struct ProseProblem<'a> {
    pub generation: super::CheckGeneration,
    pub index: usize,
    pub finding: &'a ProseFinding,
    /// Whether the word can be added to the project dictionary.
    pub can_add_word: bool,
}

impl Problems<'_> {
    pub fn len(&self) -> usize {
        self.document.len() + self.prose.map_or(0, |checked| checked.outcome.len())
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// The number of the document's own diagnostics, which come first.
    pub fn document_len(&self) -> usize {
        self.document.len()
    }
    pub fn get(&self, index: usize) -> Option<Problem<'_>> {
        let Some(index) = index.checked_sub(self.document.len()) else {
            return self.document.get(index).map(|diagnostic| Problem {
                diagnostic,
                prose: None,
            });
        };
        let checked = self.prose?;
        let diagnostic = checked.outcome.diagnostic(index)?;
        let prose = checked.outcome.finding(index).map(|finding| ProseProblem {
            generation: checked.generation,
            index,
            finding,
            can_add_word: finding.kind == ProseFindingKind::Spelling
                && checked.configuration.dictionary().is_some(),
        });
        Some(Problem { diagnostic, prose })
    }
    pub fn iter(&self) -> impl Iterator<Item = Problem<'_>> {
        (0..self.len()).filter_map(|index| self.get(index))
    }
}

impl Writer {
    /// Every problem the open document shows: listed, counted and placed at their beats.
    pub fn problems(&self) -> Problems<'_> {
        Problems {
            document: self.session.model().document().diagnostic_snapshot(),
            prose: self.current_prose(),
        }
    }
}
