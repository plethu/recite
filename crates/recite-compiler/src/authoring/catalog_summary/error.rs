use recite_core::LocaleId;

use super::CatalogIdentity;

/// Invalid input to a dialogue catalogue coverage or fallback projection.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum CatalogSummaryError {
    #[error("catalog identity must not be empty")]
    EmptyCatalogIdentity,
    #[error("expected catalogue context must not be empty")]
    EmptyExpectedContext,
    #[error("expected catalogue source text must not be empty")]
    EmptyExpectedSourceText,
    #[error("expected catalogue plural source text must not be empty")]
    EmptyExpectedPluralSourceText,
    #[error("expected catalogue repeats context `{context}` and source `{source_text}`")]
    DuplicateExpectedEntry {
        context: String,
        source_text: String,
    },
    #[error("catalogue identity `{identity:?}` is repeated")]
    DuplicateCatalog { identity: CatalogIdentity },
    #[error("catalogue locale `{locale}` is supplied more than once")]
    DuplicateCatalogLocale { locale: LocaleId },
    #[error("catalogue fallback locale `{locale}` repeats and forms a fallback cycle")]
    FallbackCycle { locale: LocaleId },
    #[error("catalogue fallback candidate `{candidate:?}` is repeated")]
    DuplicateCandidate { candidate: String },
    #[error("catalogue variant candidate policy must not be empty")]
    EmptyVariantCandidates,
    #[error("catalogue variant must not be empty")]
    EmptyVariant,
    #[error("catalogue variant `{variant}` must not contain `&`")]
    InvalidVariant { variant: String },
    #[error("source-only catalogue resolution cannot carry locale candidates")]
    SourceOnlyHasLocaleCandidates,
}
