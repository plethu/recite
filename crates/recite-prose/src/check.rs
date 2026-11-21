//! Checking a source file's prose regions and mapping findings back.

use std::collections::BTreeSet;
use std::ops::Range;

use recite_core::{
    Diagnostic, DiagnosticArgumentValue, DiagnosticCode, DiagnosticSeverity, SourceLineIndex,
    SourcePosition, SourceSpan, contracts_for_code,
};
use recite_parser::{ProseRegion, ProseRegionKind, ProseText, prose_regions};

use crate::{EngineFinding, ProseEngine, ProseFindingKind, ProseSettings, ProseSetupError, codes};

/// A replacement of source bytes. Prose fixes only ever cover bytes inside
/// one verbatim prose piece, so applying one cannot change Recite syntax.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ProseEdit {
    pub range: Range<usize>,
    pub replacement: String,
}

/// One prose finding with its structured diagnostic and safe fixes.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ProseFinding {
    pub kind: ProseFindingKind,
    /// The engine rule name, for `[prose].ignore_rules`.
    pub rule: String,
    /// The flagged source text.
    pub text: String,
    /// The flagged source byte range.
    pub range: Range<usize>,
    pub region: ProseRegionKind,
    pub stable_id: Option<String>,
    pub diagnostic: Diagnostic,
    /// Suggested replacements in engine order.
    pub fixes: Vec<ProseEdit>,
}

/// A configured engine plus the project settings that filter its findings.
pub struct ProseChecker {
    settings: ProseSettings,
    engine: Box<dyn ProseEngine>,
}

impl std::fmt::Debug for ProseChecker {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProseChecker")
            .field("settings", &self.settings)
            .finish_non_exhaustive()
    }
}

impl ProseChecker {
    /// Build the default engine with the project's language, dictionaries
    /// and suppressed rules. Building it takes a few hundred milliseconds, so
    /// long-lived clients should keep one per project.
    pub fn new(settings: ProseSettings) -> Result<Self, ProseSetupError> {
        let words = crate::dictionary::load_words(settings.dictionaries())?;
        let engine = default_engine(&settings, &words)?;
        Ok(Self::with_engine(settings, engine))
    }

    /// Use another engine, for example a test double or a server-backed one.
    #[must_use]
    pub fn with_engine(settings: ProseSettings, engine: Box<dyn ProseEngine>) -> Self {
        Self { settings, engine }
    }

    #[must_use]
    pub fn settings(&self) -> &ProseSettings {
        &self.settings
    }

    /// Check every enabled prose region of one source file. `path` labels the
    /// diagnostics. Findings are sorted by source position and code.
    pub fn check(&mut self, path: &str, source: &str) -> Vec<ProseFinding> {
        let index = SourceLineIndex::new(source);
        let mut findings = Vec::new();
        for region in prose_regions(source) {
            if !self.checks_region(&region) {
                continue;
            }
            let text = region.text(source);
            for finding in self.engine.check(&text) {
                if self.settings.ignores_rule(&finding.rule)
                    || text.overlaps_placeholder(finding.range.clone())
                {
                    continue;
                }
                if let Some(finding) = map_finding(path, source, &index, &region, &text, finding) {
                    findings.push(finding);
                }
            }
        }
        findings.sort_by(|left, right| {
            (left.range.start, left.range.end, left.kind, &left.rule).cmp(&(
                right.range.start,
                right.range.end,
                right.kind,
                &right.rule,
            ))
        });
        findings.dedup_by(|left, right| {
            left.range == right.range && left.kind == right.kind && left.rule == right.rule
        });
        findings
    }

    fn checks_region(&self, region: &ProseRegion) -> bool {
        let metadata_enabled = region
            .metadata_key()
            .is_none_or(|key| self.settings.checks_metadata_key(key));
        metadata_enabled
            && !self
                .settings
                .ignores_owner(region.speaker(), region.stable_id())
    }
}

#[cfg(feature = "harper")]
#[allow(
    clippy::unnecessary_wraps,
    reason = "matches the build without an engine"
)]
fn default_engine(
    settings: &ProseSettings,
    words: &BTreeSet<String>,
) -> Result<Box<dyn ProseEngine>, ProseSetupError> {
    Ok(Box::new(crate::HarperEngine::new(
        settings.language(),
        words.iter().map(String::as_str),
        settings.ignored_rules(),
    )))
}

#[cfg(not(feature = "harper"))]
fn default_engine(
    _settings: &ProseSettings,
    _words: &BTreeSet<String>,
) -> Result<Box<dyn ProseEngine>, ProseSetupError> {
    Err(ProseSetupError::Unavailable)
}

fn map_finding(
    path: &str,
    source: &str,
    index: &SourceLineIndex,
    region: &ProseRegion,
    text: &ProseText,
    finding: EngineFinding,
) -> Option<ProseFinding> {
    let range = text.covering_source_range(finding.range.clone());
    let flagged = source.get(range.clone())?.to_owned();
    let mut fixes = Vec::<ProseEdit>::new();
    for (checked, replacement) in finding.suggestions {
        let Some(range) = text.exact_source_range(checked) else {
            continue;
        };
        let edit = ProseEdit { range, replacement };
        if !fixes.contains(&edit) {
            fixes.push(edit);
        }
    }
    let code = match finding.kind {
        ProseFindingKind::Spelling => codes::SPELLING,
        ProseFindingKind::Grammar => codes::GRAMMAR,
        ProseFindingKind::Style => codes::STYLE,
    };
    let severity = match finding.kind {
        ProseFindingKind::Spelling | ProseFindingKind::Grammar => DiagnosticSeverity::Warning,
        ProseFindingKind::Style => DiagnosticSeverity::Information,
    };
    let message = format!("{flagged}: {} ({})", finding.message, finding.rule);
    let diagnostic = diagnostic(
        &code,
        severity,
        message,
        byte_span(path, index, source, range.clone())?,
        [
            ("text", DiagnosticArgumentValue::String(flagged.clone())),
            (
                "rule",
                DiagnosticArgumentValue::String(finding.rule.clone()),
            ),
            ("detail", DiagnosticArgumentValue::String(finding.message)),
        ],
    )?;
    Some(ProseFinding {
        kind: finding.kind,
        rule: finding.rule,
        text: flagged,
        range,
        region: region.kind(),
        stable_id: region.stable_id().map(ToOwned::to_owned),
        diagnostic,
        fixes,
    })
}

pub(crate) fn diagnostic<const N: usize>(
    code: &DiagnosticCode,
    severity: DiagnosticSeverity,
    message: String,
    span: SourceSpan,
    arguments: [(&str, DiagnosticArgumentValue); N],
) -> Option<Diagnostic> {
    let contract = contracts_for_code(code).next()?;
    Diagnostic::from_contract(severity, contract, message, span, arguments).ok()
}

/// A diagnostic span (inclusive end column) for a source byte range.
fn byte_span(
    path: &str,
    index: &SourceLineIndex,
    source: &str,
    range: Range<usize>,
) -> Option<SourceSpan> {
    let start = index.position_for_byte_offset(range.start)?;
    let last = source
        .get(range.start..range.end)?
        .char_indices()
        .next_back()
        .map(|(offset, _)| range.start + offset);
    let end = match last {
        Some(last) => Some(index.position_for_byte_offset(last)?),
        None => None,
    };
    Some(SourceSpan::new(path, start, end))
}

/// The start of a file, where setup problems are reported.
pub(crate) fn file_start(path: &str) -> Option<SourceSpan> {
    Some(SourceSpan::point(path, SourcePosition::new(1, 1).ok()?))
}
