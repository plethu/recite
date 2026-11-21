//! Spelling and grammar checking of Recite dialogue prose.
//!
//! Ownership is split three ways:
//!
//! - `recite-parser` owns *which* bytes are prose ([`recite_parser::prose_regions`])
//!   and masks inline markup, placeholders and escapes with a source map;
//! - this crate owns the project's `[prose]` settings, project dictionaries,
//!   the engine adapter and the mapping of findings to stable diagnostics
//!   (`RECITE_PROSE001`–`003`) and syntax-safe fixes;
//! - clients (the CLI, the LSP and Writer) only display findings and apply
//!   the [`ProseEdit`]s they carry.
//!
//! Checking is local and offline. The default engine is
//! [Harper](https://github.com/automattic/harper), built in with the
//! default `harper` feature. Findings are warnings or information, never
//! errors, and checking never runs as part of validation or compilation.
//!
//! # Example
//!
//! ```
//! # #[cfg(feature = "harper")] {
//! use recite_prose::{ProseChecker, ProseSettings};
//!
//! let source = ":: start default\n> speaker=rhea @intro\n  I recieved it.\n";
//! let mut checker = ProseChecker::new(ProseSettings::default()).unwrap();
//! let findings = checker.check("start.recite", source);
//! assert_eq!(findings[0].text, "recieved");
//! assert_eq!(&source[findings[0].fixes[0].range.clone()], "recieved");
//! # }
//! ```

mod cache;
mod check;
mod dictionary;
mod engine;
#[cfg(feature = "harper")]
mod harper;
mod settings;

pub use cache::ProseCheckerCache;
pub use check::{ProseChecker, ProseEdit, ProseFinding};
pub use dictionary::{dictionary_addition, parse_words};
pub use engine::{EngineFinding, ProseEngine, ProseFindingKind};
#[cfg(feature = "harper")]
pub use harper::HarperEngine;
pub use settings::{ProseLanguage, ProseSettings};

use recite_core::{Diagnostic, DiagnosticArgumentValue, DiagnosticSeverity};

/// Stable prose diagnostic codes.
pub mod codes {
    use recite_core::DiagnosticCode;

    pub const SPELLING: DiagnosticCode = DiagnosticCode::new_static("RECITE_PROSE001");
    pub const GRAMMAR: DiagnosticCode = DiagnosticCode::new_static("RECITE_PROSE002");
    pub const STYLE: DiagnosticCode = DiagnosticCode::new_static("RECITE_PROSE003");
    pub const UNSUPPORTED_LANGUAGE: DiagnosticCode = DiagnosticCode::new_static("RECITE_PROSE101");
    pub const DICTIONARY: DiagnosticCode = DiagnosticCode::new_static("RECITE_PROSE102");
    pub const UNAVAILABLE: DiagnosticCode = DiagnosticCode::new_static("RECITE_PROSE103");
}

/// A `[prose]` configuration that cannot produce a checker.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ProseSetupError {
    #[error("unsupported prose language `{0}`")]
    UnsupportedLanguage(String),
    #[error("prose dictionary `{path}`: {detail}")]
    Dictionary { path: String, detail: String },
    #[error("this build does not include a prose checker")]
    Unavailable,
}

impl ProseSetupError {
    /// The structured error reported at the start of `file`, usually the
    /// project manifest.
    #[must_use]
    pub fn diagnostic(&self, file: &str) -> Option<Diagnostic> {
        let (code, detail) = match self {
            Self::UnsupportedLanguage(language) => (codes::UNSUPPORTED_LANGUAGE, language.clone()),
            Self::Dictionary { .. } => (codes::DICTIONARY, self.to_string()),
            Self::Unavailable => (codes::UNAVAILABLE, self.to_string()),
        };
        check::diagnostic(
            &code,
            DiagnosticSeverity::Error,
            self.to_string(),
            check::file_start(file)?,
            [("detail", DiagnosticArgumentValue::String(detail))],
        )
    }
}
