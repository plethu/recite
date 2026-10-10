use crate::error::CliError;
use crate::i18n::{Messages, UiLocale};
use crate::structured::error_mapping::structured_error;
use crate::structured::errors::ErrorOperation;
use recite_config::ProjectSchemaError;
use serde_json::json;
use std::path::Path;

#[test]
fn schema_filesystem_failures_preserve_declared_and_resolved_path_authority() {
    let cases = [
        (
            ProjectSchemaError::Read {
                path: "schema.json".into(),
                source: std::io::Error::other("denied"),
            },
            json!({"category":"io","code":"read","operation":"read","path":{"encoding":"utf8","value":"schema.json"}}),
            "could not read project schema schema.json: denied",
        ),
        (
            ProjectSchemaError::InvalidPath {
                path: "declared.json".into(),
                reason: "parent component".into(),
            },
            json!({"category":"schema","code":"project_schema","operation":"resolve_schema","path":{"encoding":"utf8","value":"declared.json"}}),
            "schema path declared.json is not project-relative: parent component",
        ),
        (
            ProjectSchemaError::OutsideProject {
                declared: "alias.json".into(),
                resolved: "external.json".into(),
            },
            json!({"category":"schema","code":"project_schema","operation":"resolve_schema","path":{"encoding":"utf8","value":"alias.json"},"related_path":{"encoding":"utf8","value":"external.json"}}),
            "schema path alias.json resolves outside the project to external.json",
        ),
    ];
    let messages = Messages::load(&UiLocale::default()).unwrap();
    for (source, expected, human) in cases {
        let error = CliError::ProjectSchema { source };
        assert_eq!(
            serde_json::to_value(structured_error(
                &error,
                ErrorOperation::Compile,
                Some(Path::new("fallback.recite"))
            ))
            .unwrap(),
            expected
        );
        assert_eq!(error.to_string(), human);
        assert_eq!(error.to_user_message(&messages), human);
    }
}
