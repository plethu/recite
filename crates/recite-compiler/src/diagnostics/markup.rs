use recite_core::{Diagnostic, DiagnosticCode, RelatedSpan, SourceSpan};

const UNKNOWN_MARKUP_TAG: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE022");
const UNBALANCED_MARKUP_TAG: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE023");
const MISSING_MARKUP_CLOSING_TAG: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE024");
const INVALID_MARKUP_NESTING: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE025");

pub(crate) fn unknown_markup_tag(tag: &str, span: SourceSpan) -> Diagnostic {
    Diagnostic::error(
        UNKNOWN_MARKUP_TAG,
        format!("unknown inline markup tag `{tag}`"),
        span,
    )
    .with_help("declare the tag in the project schema manifest or remove the markup")
}

pub(crate) fn unbalanced_markup_tag(
    tag: &str,
    span: SourceSpan,
    detail: impl Into<String>,
    related_opening: Option<SourceSpan>,
) -> Diagnostic {
    let diagnostic = Diagnostic::error(
        UNBALANCED_MARKUP_TAG,
        format!("unbalanced inline markup tag `{tag}`: {}", detail.into()),
        span,
    )
    .with_help("balance inline markup tags in localisable source text");

    if let Some(opening) = related_opening {
        diagnostic.with_related([RelatedSpan::new(opening, "open markup tag is here")])
    } else {
        diagnostic
    }
}

pub(crate) fn missing_markup_closing_tag(tag: &str, span: SourceSpan) -> Diagnostic {
    Diagnostic::error(
        MISSING_MARKUP_CLOSING_TAG,
        format!("inline markup tag `{tag}` requires a closing tag"),
        span,
    )
    .with_help(format!("add `[/{}]` before the localisable text ends", tag))
}

pub(crate) fn invalid_markup_nesting(
    parent: &str,
    child: &str,
    child_span: SourceSpan,
    parent_span: SourceSpan,
) -> Diagnostic {
    Diagnostic::error(
        INVALID_MARKUP_NESTING,
        format!("inline markup tag `{parent}` cannot contain nested tag `{child}`"),
        child_span,
    )
    .with_related([RelatedSpan::new(
        parent_span,
        "non-nesting markup tag starts here",
    )])
    .with_help(format!("close `[{parent}]` before opening `[{child}]`"))
}
