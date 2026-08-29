use super::super::diagnostics::MALFORMED_SHAPE;
use super::super::raw::RawMissingMetadataContext;
use super::super::spans::ManifestSpans;
use super::super::validate::PendingDomainReference;
use crate::schema::{MissingMetadataContextPolicy, schema_diagnostic};
use crate::{Diagnostic, DiagnosticArgumentValue};

#[expect(
    clippy::too_many_arguments,
    reason = "missing-context lowering carries shared span, validation, and semantic path context"
)]
pub(super) fn lower_missing_context(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    owner: &str,
    domain_path: &[String],
    raw: Option<RawMissingMetadataContext>,
    diagnostics: &mut Vec<Diagnostic>,
    pending_domain_refs: &mut Vec<PendingDomainReference>,
) -> MissingMetadataContextPolicy {
    let Some(raw) = raw else {
        return MissingMetadataContextPolicy::Diagnostic;
    };

    let mut policy_path = domain_path.to_vec();
    policy_path.extend(["missing_context".to_owned(), "policy".to_owned()]);
    match raw.policy.as_str() {
        "diagnostic" => {
            if let Some(domain) = raw.domain {
                diagnostics.push(schema_diagnostic(
                    MALFORMED_SHAPE,
                    "diagnostic-schema-001-domain-policy-domain",
                    format!("metadata domain '{owner}' diagnostic policy must not declare domain"),
                    {
                        let mut path = domain_path.to_vec();
                        path.extend(["missing_context".to_owned(), "domain".to_owned()]);
                        spans.value_span_at(file, source, &path, &domain)
                    },
                    [
                        ("domain", DiagnosticArgumentValue::String(owner.to_owned())),
                        (
                            "policy",
                            DiagnosticArgumentValue::String("diagnostic".to_owned()),
                        ),
                    ],
                ));
            }
            MissingMetadataContextPolicy::Diagnostic
        }
        "empty" => {
            if let Some(domain) = raw.domain {
                diagnostics.push(schema_diagnostic(
                    MALFORMED_SHAPE,
                    "diagnostic-schema-001-domain-policy-domain",
                    format!("metadata domain '{owner}' empty policy must not declare domain"),
                    {
                        let mut path = domain_path.to_vec();
                        path.extend(["missing_context".to_owned(), "domain".to_owned()]);
                        spans.value_span_at(file, source, &path, &domain)
                    },
                    [
                        ("domain", DiagnosticArgumentValue::String(owner.to_owned())),
                        (
                            "policy",
                            DiagnosticArgumentValue::String("empty".to_owned()),
                        ),
                    ],
                ));
            }
            MissingMetadataContextPolicy::Empty
        }
        "fallback" => {
            let Some(domain) = raw.domain else {
                let span = spans.value_span_at(file, source, &policy_path, "fallback");
                diagnostics.push(schema_diagnostic(
                    MALFORMED_SHAPE,
                    "diagnostic-schema-001-domain-fallback-domain",
                    format!("metadata domain '{owner}' fallback policy requires domain"),
                    span,
                    [("domain", DiagnosticArgumentValue::String(owner.to_owned()))],
                ));
                return MissingMetadataContextPolicy::Diagnostic;
            };
            let mut path = domain_path.to_vec();
            path.extend(["missing_context".to_owned(), "domain".to_owned()]);
            let span = spans.value_span_at(file, source, &path, &domain);
            pending_domain_refs.push(PendingDomainReference {
                owner: format!("metadata domain '{owner}' fallback"),
                domain: domain.clone(),
                require_flat: true,
                span,
            });
            MissingMetadataContextPolicy::Fallback { domain }
        }
        other => {
            let span = spans.value_span_at(file, source, &policy_path, other);
            diagnostics.push(schema_diagnostic(
                MALFORMED_SHAPE,
                "diagnostic-schema-001-domain-policy",
                format!(
                    "metadata domain '{owner}' uses unsupported missing_context policy '{other}'"
                ),
                span,
                [
                    ("domain", DiagnosticArgumentValue::String(owner.to_owned())),
                    ("policy", DiagnosticArgumentValue::String(other.to_owned())),
                ],
            ));
            MissingMetadataContextPolicy::Diagnostic
        }
    }
}
