//! Host-neutral dialogue catalogue coverage and fallback projections.
//!
//! This module deliberately consumes the compiler's expected [`PotDocument`]
//! and core's lossless [`PoDocument`] model. It does not load files, edit PO
//! source, execute a locale provider, or render Recite-owned Fluent UI text.

mod builder;
mod coverage;
mod error;
mod resolution;
mod types;

pub use coverage::{CatalogCoverage, CatalogEntryKey, CatalogEntryStatus, TranslationStatus};
pub use error::CatalogSummaryError;
pub use resolution::{
    CatalogEntryResolution, CatalogFallbackCandidate, CatalogMatch, CatalogResolution,
    CatalogResolutionPolicy, CatalogVariant,
};
pub use types::{
    CatalogCoverageSummary, CatalogIdentity, CatalogInput, CatalogSummary, DialogueCatalog,
    DialogueCatalogInput, DialogueCatalogSummary,
};
