#![allow(clippy::expect_used)]

use recite_config::{
    CAPABILITY_REPORT_VERSION, Capability, CapabilityName, CapabilityReport, CapabilityReportError,
    CapabilityStatus, ProducerIdentity,
};
use recite_core::{
    DiagnosticCode, DiagnosticPresentation, DiagnosticPresentationId, DiagnosticRecord,
    DiagnosticSeverity, SourcePosition, SourceSpan,
};

fn capability(name: &str, status: CapabilityStatus) -> Capability {
    Capability::new(
        CapabilityName::new(name).expect("namespaced capability"),
        status,
    )
}

fn diagnostic() -> DiagnosticRecord {
    DiagnosticRecord::new(
        DiagnosticCode::new_static("RECITE_CONFIG005"),
        DiagnosticSeverity::Error,
        SourceSpan::point(
            "capability",
            SourcePosition::new(1, 1).expect("one-based position"),
        ),
        DiagnosticPresentation::new(
            DiagnosticPresentationId::new("capability-unavailable").expect("presentation ID"),
        ),
    )
}

#[test]
fn names_are_namespaced_and_reports_are_sorted_and_deduplicated() {
    let report = CapabilityReport::new(
        ProducerIdentity {
            kind: "recite-cli".to_owned(),
            id: "local".to_owned(),
        },
        [
            capability("recite.preview", CapabilityStatus::Supported),
            capability("recite.compile", CapabilityStatus::ReadOnly),
            capability("recite.preview", CapabilityStatus::Supported),
        ],
    )
    .expect("compatible duplicate");

    assert_eq!(report.version(), CAPABILITY_REPORT_VERSION);
    assert_eq!(
        report
            .capabilities()
            .iter()
            .map(|entry| entry.name().as_str())
            .collect::<Vec<_>>(),
        ["recite.compile", "recite.preview"]
    );
    assert!(report.supports(&CapabilityName::new("recite.preview").expect("name")));
    assert!(!report.supports(&CapabilityName::new("recite.compile").expect("name")));
}

#[test]
fn unavailable_status_retains_a_core_diagnostic_identity() {
    let diagnostic = diagnostic();
    let report = CapabilityReport::new(
        ProducerIdentity {
            kind: "recite-lsp".to_owned(),
            id: "workspace".to_owned(),
        },
        [capability(
            "recite.schema.edit",
            CapabilityStatus::unavailable(diagnostic.clone()),
        )],
    )
    .expect("report");

    assert_eq!(
        report.capabilities()[0].status(),
        &CapabilityStatus::Unavailable {
            diagnostic: Box::new(diagnostic),
        }
    );
}

#[test]
fn reports_round_trip_with_stable_wire_shape_and_order() {
    let report = CapabilityReport::new(
        ProducerIdentity {
            kind: "recite-cli".to_owned(),
            id: "local".to_owned(),
        },
        [capability(
            "recite.schema.edit",
            CapabilityStatus::unavailable(diagnostic()),
        )],
    )
    .expect("report");

    let encoded = serde_json::to_string(&report).expect("serialize report");
    assert_eq!(
        encoded,
        r#"{"version":1,"producer":{"kind":"recite-cli","id":"local"},"capabilities":[{"name":"recite.schema.edit","status":{"status":"unavailable","diagnostic":{"version":1,"code":"RECITE_CONFIG005","severity":"error","span":{"file":"capability","start":{"line":1,"column":1},"end":null},"presentation":{"id":"capability-unavailable","arguments":{}},"related":[],"help":null,"explanation":null,"compatibility_message":null}}}]}"#
    );
    let decoded: CapabilityReport = serde_json::from_str(&encoded).expect("deserialize report");
    assert_eq!(decoded, report);
}

#[test]
fn deserialization_revalidates_version_and_duplicate_policy() {
    let future = r#"{"version":2,"producer":{"kind":"recite","id":"test"},"capabilities":[]}"#;
    let error = serde_json::from_str::<CapabilityReport>(future).expect_err("future version");
    assert!(
        error
            .to_string()
            .contains("unsupported capability report version")
    );

    let duplicate = r#"{"version":1,"producer":{"kind":"recite","id":"test"},"capabilities":[{"name":"recite.compile","status":{"status":"supported"}},{"name":"recite.compile","status":{"status":"read_only"}}]}"#;
    let error = serde_json::from_str::<CapabilityReport>(duplicate).expect_err("conflict");
    assert!(error.to_string().contains("capability recite.compile"));

    let compatible = r#"{"version":1,"producer":{"kind":"recite","id":"test"},"capabilities":[{"name":"recite.preview","status":{"status":"supported"}},{"name":"recite.compile","status":{"status":"read_only"}},{"name":"recite.preview","status":{"status":"supported"}}]}"#;
    let decoded =
        serde_json::from_str::<CapabilityReport>(compatible).expect("compatible duplicate");
    assert_eq!(
        decoded
            .capabilities()
            .iter()
            .map(|entry| entry.name().as_str())
            .collect::<Vec<_>>(),
        ["recite.compile", "recite.preview"]
    );
}

#[test]
fn conflicting_duplicate_statuses_fail_instead_of_last_write_wins() {
    let error = CapabilityReport::new(
        ProducerIdentity {
            kind: "recite-gui".to_owned(),
            id: "standalone".to_owned(),
        },
        [
            capability("recite.schema", CapabilityStatus::Supported),
            capability("recite.schema", CapabilityStatus::ReadOnly),
        ],
    )
    .expect_err("conflict must be surfaced");

    assert!(matches!(
        error,
        CapabilityReportError::ConflictingDuplicate { .. }
    ));
}

#[test]
fn malformed_capability_names_are_rejected() {
    for name in ["compile", "Recite.compile", "recite.", "recite/compile"] {
        assert!(CapabilityName::new(name).is_err(), "{name} should fail");
    }
}
