//! Host-neutral dialogue catalogue coverage and fallback projections.
//!
//! This module deliberately consumes the compiler's expected [`PotDocument`]
//! and core's lossless [`PoDocument`] model. It does not load files, edit PO
//! source, execute a locale provider, or render Recite-owned Fluent UI text.

mod builder;
mod coverage;
mod error;
mod locale;
mod record_status;
mod resolution;
mod summary;
mod types;

pub use coverage::{CatalogCoverage, CatalogEntryKey, CatalogEntryStatus, TranslationStatus};
pub use error::CatalogSummaryError;
pub use record_status::CatalogRecordStatus;
pub use resolution::{
    CatalogEntryResolution, CatalogFallbackCandidate, CatalogMatch, CatalogResolution,
    CatalogResolutionPolicy, CatalogVariant,
};
pub use summary::{CatalogCoverageSummary, DialogueCatalogSummary};
pub use types::{
    CatalogIdentity, CatalogInput, CatalogSummary, DialogueCatalog, DialogueCatalogInput,
};
