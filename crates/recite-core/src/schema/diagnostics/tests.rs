use super::schema_diagnostic;
use crate::{DiagnosticArgumentValue, DiagnosticCode, SourcePosition, SourceSpan};

#[test]
fn factory_preserves_arbitrary_unsupported_version_lexemes() {
    for version in [
        "1.5",
        "1e-100000",
        "999999999999999999999999999999999999999999",
    ] {
        let diagnostic = schema_diagnostic(
            DiagnosticCode::new_static("RECITE_SCHEMA002"),
            "diagnostic-schema-002-unsupported-version",
            format!("unsupported schema manifest version {version}"),
            SourceSpan::point(
                "schema.json",
                SourcePosition::new(1, 1).expect("valid source position"),
            ),
            [(
                "version",
                DiagnosticArgumentValue::String(version.to_owned()),
            )],
        );

        assert_eq!(
            diagnostic
                .presentation
                .as_ref()
                .expect("structured presentation")
                .arguments()
                .get("version"),
            Some(&DiagnosticArgumentValue::String(version.to_owned()))
        );
    }
}
