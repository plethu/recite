use recite_compiler::{CompileInput, CompileOptions, CompiledAssetOutput, compile_inputs};
use recite_core::{
    BLAKE3_DIGEST_LEN, COMPILED_ASSET_FORMAT_VERSION_V0, COMPILER_COMPATIBILITY_VERSION_V0,
    CompiledAssetEncoding, CompiledAssetId, CompiledEffectMode, CompiledInspectionEncoding,
    CompiledStatementKind, CompilerVersion, SchemaFingerprint, SourceMapId, StatementIndex,
};
use serde::de::IgnoredAny;

#[path = "../../../tests/support/fixtures.rs"]
#[allow(dead_code)]
mod fixture_support;

#[test]
fn valid_fixture_compiles_to_runtime_facing_v0_tables() {
    let asset = compile_fixture("fixtures/recite/valid/core_language_spike.recite");
    let dialogue = &asset.dialogue;

    assert_eq!(
        dialogue.header.format_version,
        COMPILED_ASSET_FORMAT_VERSION_V0
    );
    assert_eq!(
        dialogue.header.compiler_compatibility_version,
        COMPILER_COMPATIBILITY_VERSION_V0
    );
    assert_eq!(
        dialogue.header.primary_encoding,
        CompiledAssetEncoding::MessagePack
    );
    assert_eq!(
        dialogue.header.inspection_encoding,
        CompiledInspectionEncoding::CompactJson
    );
    assert_eq!(dialogue.sources.len(), 1);
    assert_eq!(
        dialogue.sources[0].fingerprint.algorithm().as_str(),
        "blake3"
    );
    assert_eq!(
        dialogue.sources[0].fingerprint.digest().as_bytes().len(),
        BLAKE3_DIGEST_LEN
    );

    assert_eq!(
        dialogue
            .block_lookup
            .iter()
            .map(|entry| (entry.id.as_str(), entry.index.as_u32()))
            .collect::<Vec<_>>(),
        [("start", 0), ("work", 1)]
    );
    assert_eq!(
        dialogue
            .line_lookup
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        ["intro_001", "secret_001", "work_001"]
    );
    assert_eq!(
        dialogue
            .choice_lookup
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        ["ask_work"]
    );
    assert_eq!(
        dialogue
            .speakers
            .iter()
            .map(|speaker| speaker.id.as_str())
            .collect::<Vec<_>>(),
        ["narrator", "hazel"]
    );

    let start_block = &dialogue.blocks[0];
    assert_eq!(start_block.statements.start, StatementIndex::new(0));
    assert_eq!(start_block.statements.len, 4);

    let CompiledStatementKind::Prompt { line, choices } = &dialogue.statements[0].kind else {
        panic!("expected prompt statement");
    };
    assert_eq!(line.as_ref().map(|index| index.as_u32()), Some(0));
    assert_eq!(choices.start.as_u32(), 0);
    assert_eq!(choices.len, 1);

    let CompiledStatementKind::If {
        then_statements, ..
    } = &dialogue.statements[1].kind
    else {
        panic!("expected if statement");
    };
    assert_eq!(then_statements.start.as_u32(), 4);
    assert_eq!(then_statements.len, 1);
    assert!(matches!(
        dialogue.statements[2].kind,
        CompiledStatementKind::Effect(_)
    ));
    assert!(matches!(
        dialogue.statements[3].kind,
        CompiledStatementKind::End
    ));
    assert!(matches!(
        dialogue.statements[4].kind,
        CompiledStatementKind::Line(_)
    ));

    let intro = &dialogue.lines[0];
    assert_eq!(intro.id.as_str(), "intro_001");
    assert_eq!(intro.speaker.map(|index| index.as_u32()), Some(1));
    assert_eq!(
        metadata_keys_for(dialogue, intro.metadata),
        ["mood", "mood"]
    );

    assert_eq!(dialogue.effects.len(), 1);
    assert_eq!(dialogue.effects[0].mode, CompiledEffectMode::Deferred);
    assert_eq!(dialogue.effects[0].function, "advance_thread");
    assert!(
        dialogue
            .source_maps
            .iter()
            .all(|source_map| source_map.span.start.line() != 2)
    );
}

#[test]
fn compilation_output_is_deterministic_for_identical_inputs() {
    let first = compile_fixture("fixtures/recite/valid/core_language_spike.recite");
    let second = compile_fixture("fixtures/recite/valid/core_language_spike.recite");

    assert_eq!(first.messagepack, second.messagepack);
    assert_eq!(first.inspection_json, second.inspection_json);

    type TopLevel = (
        Vec<IgnoredAny>,
        IgnoredAny,
        IgnoredAny,
        IgnoredAny,
        IgnoredAny,
        IgnoredAny,
        IgnoredAny,
        IgnoredAny,
        IgnoredAny,
        IgnoredAny,
        IgnoredAny,
        IgnoredAny,
        IgnoredAny,
        IgnoredAny,
    );
    let decoded: TopLevel = rmp_serde::from_slice(&first.messagepack).expect("messagepack decodes");
    assert_eq!(decoded.0.len(), 8);
}

#[test]
fn compilation_order_is_canonical_for_project_inputs() {
    let start = CompileInput::new(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line\n",
            "  Start.\n",
            "-> dialogue/next.recite::next\n",
        ),
    );
    let next = CompileInput::new(
        "dialogue/next.recite",
        concat!(":: next\n", "> next_line\n", "  Next.\n", "-> END\n",),
    );

    let forward = compile_inputs([start.clone(), next.clone()], options())
        .expect("compile succeeds")
        .asset
        .expect("asset emitted");
    let reverse = compile_inputs([next, start], options())
        .expect("compile succeeds")
        .asset
        .expect("asset emitted");

    assert_eq!(forward.messagepack, reverse.messagepack);
    assert_eq!(
        forward
            .dialogue
            .sources
            .iter()
            .map(|source| source.path.as_str())
            .collect::<Vec<_>>(),
        ["dialogue/next.recite", "dialogue/start.recite"]
    );
}

#[test]
fn malformed_or_invalid_content_returns_diagnostics_without_asset() {
    let parse_failure = compile_inputs(
        [CompileInput::new(
            "dialogue/bad.recite",
            concat!(
                ":: start default\n",
                "> line\n",
                "  Text.\n",
                "    Mixed indent.\n",
            ),
        )],
        options(),
    )
    .expect("content diagnostics are not hard errors");

    assert!(parse_failure.asset.is_none());
    assert_eq!(
        parse_failure.diagnostics[0].code.as_str(),
        "RECITE_PARSE007"
    );

    let validation_failure = compile_inputs(
        [CompileInput::new(
            "dialogue/bad.recite",
            concat!(":: start default\n", "? missing_target\n", "  Choose.\n",),
        )],
        options(),
    )
    .expect("validation diagnostics are not hard errors");

    assert!(validation_failure.asset.is_none());
    assert_eq!(
        validation_failure.diagnostics[0].code.as_str(),
        "RECITE_VALIDATE012"
    );
}

#[test]
fn compact_json_inspection_output_is_snapshot_stable() {
    let asset = compile_fixture("fixtures/recite/valid/core_language_spike.recite");

    fixture_support::assert_text_snapshot(
        &asset.inspection_json,
        "compiled_asset_v0_valid_fixture_json".to_owned(),
    );
}

fn compile_fixture(path: &str) -> CompiledAssetOutput {
    let source = fixture_support::fixture_source(path);
    let report = compile_inputs([CompileInput::new(path, source)], options())
        .expect("fixture compile does not hard-fail");

    assert!(
        report.diagnostics.is_empty(),
        "fixture should compile without diagnostics: {:?}",
        report.diagnostics
    );

    report.asset.expect("valid fixture emits an asset")
}

fn options() -> CompileOptions {
    CompileOptions::new(
        CompilerVersion::new("0.0.1").expect("valid compiler version"),
        CompiledAssetId::new("dialogue/main.recitec").expect("valid asset id"),
        SourceMapId::new("dialogue/main.recitec.map").expect("valid source map id"),
        SchemaFingerprint::NoSchema,
    )
}

fn metadata_keys_for(
    dialogue: &recite_core::CompiledDialogue,
    range: recite_core::MetadataRange,
) -> Vec<&str> {
    let start = range.start.as_u32() as usize;
    let end = start + range.len as usize;
    dialogue.metadata[start..end]
        .iter()
        .map(|entry| entry.key.as_str())
        .collect()
}
