use recite_core::{
    COMPILED_ASSET_FORMAT_VERSION_V0, COMPILER_COMPATIBILITY_VERSION_V0, CompiledAssetDecodeError,
    CompiledDialogue, Diagnostic, DiagnosticArgumentValue, SourceSpan,
    decode_compiled_dialogue_messagepack,
    project::{MALFORMED_COMPILED_ASSET, STALE_COMPILER_COMPATIBILITY, UNSUPPORTED_ASSET_VERSION},
};

use super::project_diagnostics::project_diagnostic;

pub(super) fn decode_project_asset(
    bytes: &[u8],
    scene_id: &str,
    asset_name: &str,
    asset_span: SourceSpan,
) -> Result<CompiledDialogue, Box<Diagnostic>> {
    match decode_compiled_dialogue_messagepack(bytes) {
        Ok(asset) => Ok(asset),
        Err(CompiledAssetDecodeError::UnsupportedFormat {
            format_version,
            compiler_compatibility_version,
        }) if format_version == COMPILED_ASSET_FORMAT_VERSION_V0
            && compiler_compatibility_version != COMPILER_COMPATIBILITY_VERSION_V0 =>
        {
            Err(Box::new(project_diagnostic(
                &STALE_COMPILER_COMPATIBILITY,
                "diagnostic-fresh-003",
                format!(
                    "compiled asset '{}' uses compiler compatibility version {}, expected {}",
                    asset_name, compiler_compatibility_version, COMPILER_COMPATIBILITY_VERSION_V0
                ),
                asset_span,
                [
                    (
                        "asset",
                        DiagnosticArgumentValue::String(asset_name.to_owned()),
                    ),
                    (
                        "version",
                        DiagnosticArgumentValue::Integer(i64::from(compiler_compatibility_version)),
                    ),
                    (
                        "expected",
                        DiagnosticArgumentValue::Integer(i64::from(
                            COMPILER_COMPATIBILITY_VERSION_V0,
                        )),
                    ),
                ],
            )))
        }
        Err(CompiledAssetDecodeError::UnsupportedFormat { format_version, .. }) => {
            Err(Box::new(project_diagnostic(
                &UNSUPPORTED_ASSET_VERSION,
                "diagnostic-project-007",
                format!(
                    "compiled asset '{}' uses unsupported format version {}",
                    asset_name, format_version
                ),
                asset_span,
                [
                    (
                        "asset",
                        DiagnosticArgumentValue::String(asset_name.to_owned()),
                    ),
                    (
                        "version",
                        DiagnosticArgumentValue::Integer(i64::from(format_version)),
                    ),
                ],
            )))
        }
        Err(error) => Err(Box::new(project_diagnostic(
            &MALFORMED_COMPILED_ASSET,
            "diagnostic-project-007-malformed",
            format!(
                "scene '{}' references malformed compiled asset '{}': {error}",
                scene_id, asset_name
            ),
            asset_span,
            [
                (
                    "scene_id",
                    DiagnosticArgumentValue::String(scene_id.to_owned()),
                ),
                (
                    "asset",
                    DiagnosticArgumentValue::String(asset_name.to_owned()),
                ),
                ("detail", DiagnosticArgumentValue::String(error.to_string())),
            ],
        ))),
    }
}
