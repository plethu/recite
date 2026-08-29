use std::{collections::BTreeMap, fs};

use recite_compiler::{CompileInput, compile_inputs};
use recite_core::{
    Diagnostic, DiagnosticArgumentValue, DiagnosticSeverity, ProjectManifest, SchemaFingerprint,
    SourcePosition, SourceSpan, decode_compiled_dialogue_messagepack,
    project::{
        MALFORMED_COMPILED_ASSET, MISSING_COMPILED_ASSET, STALE_COMPILER_COMPATIBILITY,
        UNSUPPORTED_ASSET_VERSION,
    },
};
use tempfile::TempDir;

use super::inputs::compile_options;
use super::project::validate_project_asset_freshness;
use super::project_diagnostics::project_diagnostic;

fn span() -> SourceSpan {
    SourceSpan::point(
        "recite.project.toml",
        SourcePosition::new(1, 1).expect("valid source position"),
    )
}

fn assert_recordable(
    diagnostic: Diagnostic,
    presentation_id: &str,
    arguments: BTreeMap<String, DiagnosticArgumentValue>,
) {
    assert_eq!(diagnostic.severity, DiagnosticSeverity::Error);
    let presentation = diagnostic.presentation.as_ref().expect("presentation");
    assert_eq!(presentation.id().as_str(), presentation_id);
    assert_eq!(presentation.arguments(), &arguments);
    diagnostic.record().expect("structured diagnostic record");
}

fn manifest_source(asset: &str) -> recite_core::ProjectManifestSource {
    let source = format!(
        "[[scenes]]\nid = \"scene.start\"\nasset = \"{asset}\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n"
    );
    let report = ProjectManifest::load_str_with_spans("recite.project.toml", &source);
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    report.source.expect("manifest source")
}

fn producer_diagnostic(root: &TempDir, asset: &str) -> Diagnostic {
    let source = manifest_source(asset);
    let mut diagnostics =
        validate_project_asset_freshness(root.path(), &source, Some(SchemaFingerprint::NoSchema))
            .expect("asset freshness validation");
    assert_eq!(diagnostics.len(), 1);
    diagnostics.remove(0)
}

#[test]
fn cli_project_missing_asset_uses_exact_typed_contract() {
    assert_recordable(
        project_diagnostic(
            &MISSING_COMPILED_ASSET,
            "diagnostic-project-003",
            "missing asset",
            span(),
            [
                (
                    "scene_id",
                    DiagnosticArgumentValue::String("scene.start".to_owned()),
                ),
                (
                    "asset",
                    DiagnosticArgumentValue::String("dialogue.recitec".to_owned()),
                ),
            ],
        ),
        "diagnostic-project-003",
        BTreeMap::from([
            (
                "scene_id".to_owned(),
                DiagnosticArgumentValue::String("scene.start".to_owned()),
            ),
            (
                "asset".to_owned(),
                DiagnosticArgumentValue::String("dialogue.recitec".to_owned()),
            ),
        ]),
    );
}

#[test]
fn cli_project_malformed_asset_uses_exact_typed_contract() {
    assert_recordable(
        project_diagnostic(
            &MALFORMED_COMPILED_ASSET,
            "diagnostic-project-007-malformed",
            "malformed asset",
            span(),
            [
                (
                    "scene_id",
                    DiagnosticArgumentValue::String("scene.start".to_owned()),
                ),
                (
                    "asset",
                    DiagnosticArgumentValue::String("dialogue.recitec".to_owned()),
                ),
                (
                    "detail",
                    DiagnosticArgumentValue::String("bad bytes".to_owned()),
                ),
            ],
        ),
        "diagnostic-project-007-malformed",
        BTreeMap::from([
            (
                "scene_id".to_owned(),
                DiagnosticArgumentValue::String("scene.start".to_owned()),
            ),
            (
                "asset".to_owned(),
                DiagnosticArgumentValue::String("dialogue.recitec".to_owned()),
            ),
            (
                "detail".to_owned(),
                DiagnosticArgumentValue::String("bad bytes".to_owned()),
            ),
        ]),
    );
}

#[test]
fn cli_freshness_compatibility_uses_exact_typed_contract() {
    assert_recordable(
        project_diagnostic(
            &STALE_COMPILER_COMPATIBILITY,
            "diagnostic-fresh-003",
            "stale compiler compatibility",
            span(),
            [
                (
                    "asset",
                    DiagnosticArgumentValue::String("dialogue.recitec".to_owned()),
                ),
                ("version", DiagnosticArgumentValue::Integer(1)),
                ("expected", DiagnosticArgumentValue::Integer(0)),
            ],
        ),
        "diagnostic-fresh-003",
        BTreeMap::from([
            (
                "asset".to_owned(),
                DiagnosticArgumentValue::String("dialogue.recitec".to_owned()),
            ),
            ("version".to_owned(), DiagnosticArgumentValue::Integer(1)),
            ("expected".to_owned(), DiagnosticArgumentValue::Integer(0)),
        ]),
    );
}

#[test]
fn missing_asset_producer_emits_exact_recordable_project003() {
    let root = TempDir::new().expect("tempdir");
    let diagnostic = producer_diagnostic(&root, "dialogue.recitec");

    assert_recordable(
        diagnostic,
        "diagnostic-project-003",
        BTreeMap::from([
            (
                "scene_id".to_owned(),
                DiagnosticArgumentValue::String("scene.start".to_owned()),
            ),
            (
                "asset".to_owned(),
                DiagnosticArgumentValue::String("dialogue.recitec".to_owned()),
            ),
        ]),
    );
}

#[test]
fn malformed_asset_producer_emits_exact_recordable_project007() {
    let root = TempDir::new().expect("tempdir");
    let bytes = [1, 2, 3];
    fs::write(root.path().join("dialogue.recitec"), bytes).expect("malformed asset");
    let detail = decode_compiled_dialogue_messagepack(&bytes)
        .expect_err("malformed bytes must fail to decode")
        .to_string();
    let diagnostic = producer_diagnostic(&root, "dialogue.recitec");

    assert_recordable(
        diagnostic,
        "diagnostic-project-007-malformed",
        BTreeMap::from([
            (
                "scene_id".to_owned(),
                DiagnosticArgumentValue::String("scene.start".to_owned()),
            ),
            (
                "asset".to_owned(),
                DiagnosticArgumentValue::String("dialogue.recitec".to_owned()),
            ),
            ("detail".to_owned(), DiagnosticArgumentValue::String(detail)),
        ]),
    );
}

#[test]
fn unsupported_format_producer_emits_exact_recordable_project007() {
    let root = TempDir::new().expect("tempdir");
    let asset_path = root.path().join("dialogue.recitec");
    let report = compile_inputs(
        [CompileInput::new(
            "dialogue.recite",
            concat!(
                ":: start default speaker=hazel\n",
                "> intro@11111111111111111111\n",
                "  Hello.\n",
                "-> END\n",
            ),
        )],
        compile_options(&asset_path, None).expect("compile options"),
    )
    .expect("compile asset");
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    let mut bytes = report.asset.expect("compiled asset").messagepack;
    assert!(bytes.len() >= 6, "compiled asset header must be present");
    assert_eq!(bytes[4], 0, "compiled asset starts at format version v0");
    bytes[4] = 1;
    fs::write(&asset_path, bytes).expect("unsupported format asset");
    let diagnostic = producer_diagnostic(&root, "dialogue.recitec");

    assert_eq!(diagnostic.code, UNSUPPORTED_ASSET_VERSION);
    assert_recordable(
        diagnostic,
        "diagnostic-project-007",
        BTreeMap::from([
            (
                "asset".to_owned(),
                DiagnosticArgumentValue::String("dialogue.recitec".to_owned()),
            ),
            ("version".to_owned(), DiagnosticArgumentValue::Integer(1)),
        ]),
    );
}

#[test]
fn stale_compatibility_producer_emits_exact_recordable_fresh003() {
    let root = TempDir::new().expect("tempdir");
    let asset_path = root.path().join("dialogue.recitec");
    let report = compile_inputs(
        [CompileInput::new(
            "dialogue.recite",
            concat!(
                ":: start default speaker=hazel\n",
                "> intro@11111111111111111111\n",
                "  Hello.\n",
                "-> END\n",
            ),
        )],
        compile_options(&asset_path, None).expect("compile options"),
    )
    .expect("compile asset");
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    let mut bytes = report.asset.expect("compiled asset").messagepack;
    assert!(
        bytes.len() >= 6,
        "compiled dialogue header must include the compatibility field"
    );
    assert_eq!(
        &bytes[..6],
        &[0xdc, 0, 17, 0x98, 0, 0],
        "compiled dialogue header layout changed"
    );
    bytes[5] = 1;
    fs::write(&asset_path, bytes).expect("stale asset");
    let diagnostic = producer_diagnostic(&root, "dialogue.recitec");

    assert_recordable(
        diagnostic,
        "diagnostic-fresh-003",
        BTreeMap::from([
            (
                "asset".to_owned(),
                DiagnosticArgumentValue::String("dialogue.recitec".to_owned()),
            ),
            ("version".to_owned(), DiagnosticArgumentValue::Integer(1)),
            ("expected".to_owned(), DiagnosticArgumentValue::Integer(0)),
        ]),
    );
}
