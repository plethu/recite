#[path = "project_test_support/freshness.rs"]
mod freshness_support;
mod project_test_support;

use std::collections::BTreeMap;

use freshness_support::{asset_with, manifest_source, source_fingerprint};
use project_test_support::{assert_diagnostic, string};

fn integer(value: i64) -> recite_core::DiagnosticArgumentValue {
    recite_core::DiagnosticArgumentValue::Integer(value)
}
use recite_core::{
    CompiledSourceFile, ContentFingerprint, ProjectFreshnessInput, SchemaFingerprint,
    validate_project_freshness_source,
};

#[test]
fn freshness_producers_preserve_variant_arguments_and_order() {
    let source = manifest_source();
    let stale_schema = ContentFingerprint::blake3([7; 32])
        .unwrap_or_else(|error| panic!("valid fingerprint: {error}"));
    let asset = asset_with(
        1,
        1,
        SchemaFingerprint::Fingerprint(stale_schema),
        vec![
            CompiledSourceFile {
                path: "dialogue.recite".to_owned(),
                fingerprint: source_fingerprint("old source"),
            },
            CompiledSourceFile {
                path: "missing.recite".to_owned(),
                fingerprint: source_fingerprint("missing source"),
            },
        ],
        &["other"],
        &["other"],
    );
    let mut current_sources = BTreeMap::new();
    current_sources.insert("dialogue.recite", Some("new source"));
    current_sources.insert("missing.recite", None);
    let diagnostics = validate_project_freshness_source(
        &source,
        ProjectFreshnessInput {
            scene_index: 0,
            scene: &source.manifest().scenes[0],
            asset: &asset,
            current_sources,
            current_schema_fingerprint: Some(SchemaFingerprint::Fingerprint(source_fingerprint(
                "current schema",
            ))),
        },
    );
    assert_eq!(diagnostics.len(), 7);
    assert_eq!(diagnostics[0].span, source.scene_key_span(0, "block"));
    assert_eq!(
        diagnostics[1].span,
        source.scene_key_span(0, "participants")
    );
    for diagnostic in &diagnostics[2..] {
        assert_eq!(diagnostic.span, source.scene_key_span(0, "asset"));
    }

    assert_diagnostic(
        &diagnostics[0],
        "RECITE_PROJECT004",
        "diagnostic-project-004",
        &[("scene_id", string("opening")), ("block", string("start"))],
    );
    assert_diagnostic(
        &diagnostics[1],
        "RECITE_PROJECT008",
        "diagnostic-project-008-compiled-asset",
        &[
            ("scene_id", string("opening")),
            ("participant", string("hazel")),
            ("asset", string("dialogue.recitec")),
        ],
    );
    assert_diagnostic(
        &diagnostics[2],
        "RECITE_FRESH001",
        "diagnostic-fresh-001",
        &[
            ("asset", string("dialogue.recitec")),
            ("source", string("dialogue.recite")),
        ],
    );
    assert_diagnostic(
        &diagnostics[3],
        "RECITE_PROJECT006",
        "diagnostic-project-006",
        &[
            ("asset", string("dialogue.recitec")),
            ("source", string("missing.recite")),
        ],
    );
    assert_diagnostic(
        &diagnostics[4],
        "RECITE_FRESH002",
        "diagnostic-fresh-002",
        &[("asset", string("dialogue.recitec"))],
    );
    assert_diagnostic(
        &diagnostics[5],
        "RECITE_FRESH003",
        "diagnostic-fresh-003",
        &[
            ("asset", string("dialogue.recitec")),
            ("version", integer(1)),
            ("expected", integer(0)),
        ],
    );
    assert_diagnostic(
        &diagnostics[6],
        "RECITE_PROJECT007",
        "diagnostic-project-007",
        &[
            ("asset", string("dialogue.recitec")),
            ("version", integer(1)),
        ],
    );
}

#[test]
fn freshness_stale_source_branch_uses_exact_span() {
    let source = manifest_source();
    let asset = asset_with(
        0,
        0,
        SchemaFingerprint::NoSchema,
        vec![CompiledSourceFile {
            path: "dialogue.recite".to_owned(),
            fingerprint: source_fingerprint("old"),
        }],
        &["start"],
        &["hazel"],
    );
    let mut current_sources = BTreeMap::new();
    current_sources.insert("dialogue.recite", Some("new"));
    let diagnostics = validate_project_freshness_source(
        &source,
        ProjectFreshnessInput {
            scene_index: 0,
            scene: &source.manifest().scenes[0],
            asset: &asset,
            current_sources,
            current_schema_fingerprint: None,
        },
    );
    assert_eq!(diagnostics.len(), 1);
    assert_diagnostic(
        &diagnostics[0],
        "RECITE_FRESH001",
        "diagnostic-fresh-001",
        &[
            ("asset", string("dialogue.recitec")),
            ("source", string("dialogue.recite")),
        ],
    );
    assert_eq!(diagnostics[0].span.start.line(), 3);
    assert_eq!(diagnostics[0].span.start.column(), 1);
}
