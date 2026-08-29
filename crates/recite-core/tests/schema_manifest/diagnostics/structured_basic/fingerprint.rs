use recite_core::{DiagnosticArgumentValue, DiagnosticCode, load_schema_manifest_str};
use std::collections::BTreeMap;

#[test]
fn manifest_content_fingerprint_failures_have_exact_structured_records() {
    let cases = [
        (
            "",
            "00",
            "diagnostic-schema-001-producer-content-fingerprint-empty-algorithm",
            "manifest content_fingerprint is invalid: FingerprintAlgorithm must not be empty",
            Vec::new(),
        ),
        (
            "blake3",
            "0",
            "diagnostic-schema-001-producer-content-fingerprint-blake3-hex-shape",
            "manifest content_fingerprint is invalid: blake3 producer fingerprint must be even-length hex",
            Vec::new(),
        ),
        (
            "blake3",
            "zz",
            "diagnostic-schema-001-producer-content-fingerprint-blake3-hex-data",
            "manifest content_fingerprint is invalid: blake3 producer fingerprint must be hex",
            Vec::new(),
        ),
        (
            "sha256",
            "",
            "diagnostic-schema-001-producer-content-fingerprint-empty-digest",
            "manifest content_fingerprint is invalid: FingerprintDigest must not be empty",
            Vec::new(),
        ),
        (
            "blake3",
            "00",
            "diagnostic-schema-001-producer-content-fingerprint-blake3-digest-length",
            "manifest content_fingerprint is invalid: blake3 fingerprint digest must be 32 bytes, got 1",
            vec![("actual", DiagnosticArgumentValue::Integer(1))],
        ),
    ];

    for (algorithm, value, presentation_id, compatibility_message, arguments) in cases {
        assert_eq!(
            recite_core::producer_content_fingerprint(algorithm, value)
                .expect_err("invalid fingerprint"),
            compatibility_message
                .strip_prefix("manifest content_fingerprint is invalid: ")
                .expect("fingerprint compatibility message prefix")
        );
        let source = format!(
            r#"{{"schema_version":1,"content_fingerprint":{{"algorithm":"{algorithm}","value":"{value}"}}}}"#
        );
        let report = load_schema_manifest_str("fingerprint.json", &source);
        assert!(report.schema.is_none());
        assert_eq!(report.diagnostics.len(), 1);
        let diagnostic = &report.diagnostics[0];
        assert_eq!(
            diagnostic.code,
            DiagnosticCode::new_static("RECITE_SCHEMA001")
        );
        assert!(diagnostic.related.is_empty());
        assert!(diagnostic.help.is_none());
        assert_eq!(diagnostic.span.file, "fingerprint.json");
        assert_eq!(diagnostic.span.start.line(), 1);
        assert_eq!(diagnostic.span.start.column(), 21);
        assert_eq!(
            diagnostic
                .span
                .end
                .map(|position| (position.line(), position.column())),
            Some((1, 42))
        );
        let presentation = diagnostic
            .presentation
            .as_ref()
            .expect("structured fingerprint presentation");
        assert_eq!(presentation.id().as_str(), presentation_id);
        assert_eq!(
            presentation.arguments(),
            &arguments
                .into_iter()
                .map(|(name, value)| (name.to_owned(), value))
                .collect::<BTreeMap<_, _>>()
        );
        diagnostic
            .record()
            .expect("recordable fingerprint diagnostic");
    }
}
