//! Position-only revisions keep project meaning and declaration order intact.
use super::ProjectFacts;
use recite_core::{SourceSpan, ast::DivertTarget};

impl ProjectFacts {
    pub(crate) fn same_semantics(&self, next: &Self) -> bool {
        self.path == next.path
            && self.participation == next.participation
            && self.blocks.len() == next.blocks.len()
            && self
                .blocks
                .iter()
                .zip(&next.blocks)
                .all(|(a, b)| a.id == b.id && a.default == b.default)
            && self.passages.len() == next.passages.len()
            && self
                .passages
                .iter()
                .zip(&next.passages)
                .all(|(a, b)| a.identity == b.identity && a.frozen == b.frozen)
            && self.references.len() == next.references.len()
            && self.references.iter().zip(&next.references).all(|(a, b)| {
                match (&a.target, &b.target) {
                    (DivertTarget::End, DivertTarget::End) => true,
                    (DivertTarget::Block(a), DivertTarget::Block(b)) => {
                        a.file == b.file && a.block_id == b.block_id
                    }
                    _ => false,
                }
            })
    }

    /// Primary project diagnostic locations. Reference token spans are not used
    /// by project diagnostics; an unrecognised location forces full validation.
    pub(crate) fn diagnostic_spans(&self) -> impl Iterator<Item = &SourceSpan> {
        self.blocks
            .iter()
            .map(|item| &item.span)
            .chain(self.passages.iter().map(|item| &item.span))
            .chain(self.references.iter().map(|item| &item.span))
    }
}
