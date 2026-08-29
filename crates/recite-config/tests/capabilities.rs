#![allow(clippy::expect_used)]

use recite_config::{
    CAPABILITY_REPORT_VERSION, Capability, CapabilityName, CapabilityReport, CapabilityReportError,
    CapabilityStatus, ProducerIdentity,
};
use recite_core::DiagnosticCode;

fn capability(name: &str, status: CapabilityStatus) -> Capability {
    Capability::new(
        CapabilityName::new(name).expect("namespaced capability"),
        status,
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

    assert_eq!(report.version, CAPABILITY_REPORT_VERSION);
    assert_eq!(
        report
            .capabilities
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>(),
        ["recite.compile", "recite.preview"]
    );
    assert!(report.supports(&CapabilityName::new("recite.preview").expect("name")));
    assert!(!report.supports(&CapabilityName::new("recite.compile").expect("name")));
}

#[test]
fn unavailable_status_retains_a_core_diagnostic_identity() {
    let code = DiagnosticCode::new_static("RECITE_CONFIG005");
    let report = CapabilityReport::new(
        ProducerIdentity {
            kind: "recite-lsp".to_owned(),
            id: "workspace".to_owned(),
        },
        [capability(
            "recite.schema.edit",
            CapabilityStatus::unavailable(code.clone()),
        )],
    )
    .expect("report");

    assert_eq!(
        report.capabilities[0].status,
        CapabilityStatus::Unavailable { diagnostic: code }
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
