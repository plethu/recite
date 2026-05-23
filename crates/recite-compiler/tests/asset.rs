use recite_compiler::{
    CompileInput, CompileOptions, CompiledAssetOutput, compile_inputs, compile_inputs_with_schema,
};
use recite_core::{
    BLAKE3_DIGEST_LEN, COMPILED_ASSET_FORMAT_VERSION_V0, COMPILER_COMPATIBILITY_VERSION_V0,
    CompiledAssetDecodeError, CompiledAssetEncoding, CompiledAssetId, CompiledEffectMode,
    CompiledInspectionEncoding, CompiledStatementKind, CompilerVersion, SchemaFingerprint,
    SourceMapId, StatementIndex, canonical_source_fingerprint,
    decode_compiled_dialogue_messagepack, load_schema_manifest_str,
};
use serde::Serialize;
use serde::de::IgnoredAny;
use serde::ser::SerializeTuple;
use serde_bytes::Bytes;

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
        dialogue.sources[0].fingerprint,
        canonical_source_fingerprint(&fixture_support::fixture_source(
            "fixtures/recite/valid/core_language_spike.recite"
        ))
    );

    assert_eq!(
        dialogue
            .block_lookup
            .iter()
            .map(|entry| (entry.id.as_str(), entry.index.as_u32()))
            .collect::<Vec<_>>(),
        [("start", 0), ("work", 1)]
    );
    assert_eq!(dialogue.default_block.as_u32(), 0);
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
    type TopLevelSpeakers = (
        IgnoredAny,
        IgnoredAny,
        IgnoredAny,
        IgnoredAny,
        IgnoredAny,
        IgnoredAny,
        IgnoredAny,
        IgnoredAny,
        Vec<(String,)>,
        IgnoredAny,
        IgnoredAny,
        IgnoredAny,
        IgnoredAny,
        IgnoredAny,
        IgnoredAny,
    );
    let decoded: TopLevelSpeakers =
        rmp_serde::from_slice(&asset.messagepack).expect("speaker rows use fixed arrays");
    assert_eq!(decoded.8, [("narrator".to_owned(),), ("hazel".to_owned(),)]);

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
        IgnoredAny,
        u32,
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
    assert_eq!(decoded.1, 0);
}

#[test]
fn compiler_generated_messagepack_decodes_to_compiled_dialogue() {
    let asset = compile_fixture("fixtures/recite/valid/core_language_spike.recite");

    let decoded = decode_compiled_dialogue_messagepack(&asset.messagepack)
        .expect("compiler-generated MessagePack decodes");

    assert_eq!(decoded, asset.dialogue);
}

#[test]
fn compiler_generated_messagepack_decodes_punctuation_metadata_keys() {
    let report = compile_inputs(
        [CompileInput::new(
            "dialogue/main.recite",
            concat!(
                ":: start default\n",
                "> intro ui:portrait=flat\n",
                "  Hello.\n",
                "-> END\n",
            ),
        )],
        options(),
    )
    .expect("compile does not hard-fail");

    assert!(
        report.diagnostics.is_empty(),
        "punctuation metadata key should compile without schema diagnostics: {:?}",
        report.diagnostics
    );
    let asset = report.asset.expect("asset emitted");
    let decoded = decode_compiled_dialogue_messagepack(&asset.messagepack)
        .expect("compiler-generated MessagePack decodes");

    assert_eq!(decoded, asset.dialogue);
    assert_eq!(decoded.metadata[0].key, "ui:portrait");
}

#[test]
fn decode_rejects_unexpected_top_level_array_length() {
    let bytes = rmp_serde::to_vec(&(valid_header(), 0_u32)).expect("test wire encodes");

    let error = decode_compiled_dialogue_messagepack(&bytes).expect_err("arity is rejected");

    assert!(matches!(error, CompiledAssetDecodeError::MalformedAsset(_)));
}

#[test]
fn decode_rejects_unknown_wire_tags() {
    let mut asset = valid_wire_asset();
    asset.header.primary_encoding = Tagged::nil(99);
    let bytes = rmp_serde::to_vec(&asset).expect("test wire encodes");

    let error = decode_compiled_dialogue_messagepack(&bytes).expect_err("unknown tag is rejected");

    assert!(matches!(
        error,
        CompiledAssetDecodeError::MalformedAsset(message) if message.contains("unknown asset encoding tag 99")
    ));
}

#[test]
fn decode_reports_unsupported_format_before_v0_body_validation() {
    let mut asset = valid_wire_asset();
    asset.header.format_version = COMPILED_ASSET_FORMAT_VERSION_V0 + 1;
    asset.header.primary_encoding = Tagged::nil(99);
    let bytes = rmp_serde::to_vec(&asset).expect("test wire encodes");

    let error =
        decode_compiled_dialogue_messagepack(&bytes).expect_err("unsupported format rejected");

    assert!(matches!(
        error,
        CompiledAssetDecodeError::UnsupportedFormat {
            format_version,
            compiler_compatibility_version
        } if format_version == COMPILED_ASSET_FORMAT_VERSION_V0 + 1
            && compiler_compatibility_version == COMPILER_COMPATIBILITY_VERSION_V0
    ));
}

#[test]
fn decode_reports_unsupported_format_before_future_body_shape_validation() {
    let bytes = rmp_serde::to_vec(&((
        COMPILED_ASSET_FORMAT_VERSION_V0 + 1,
        COMPILER_COMPATIBILITY_VERSION_V0,
    ),))
    .expect("future-shaped test wire encodes");

    let error =
        decode_compiled_dialogue_messagepack(&bytes).expect_err("unsupported format rejected");

    assert!(matches!(
        error,
        CompiledAssetDecodeError::UnsupportedFormat {
            format_version,
            compiler_compatibility_version
        } if format_version == COMPILED_ASSET_FORMAT_VERSION_V0 + 1
            && compiler_compatibility_version == COMPILER_COMPATIBILITY_VERSION_V0
    ));
}

#[test]
fn decode_rejects_invalid_fingerprint_digest_length() {
    let mut asset = valid_wire_asset();
    asset.header.schema_fingerprint = Tagged::payload(
        recite_core::V0_SCHEMA_FINGERPRINT_TAG_FINGERPRINT,
        WireFingerprint {
            algorithm: "blake3",
            digest: Bytes::new(&SHORT_DIGEST),
        },
    );
    let bytes = rmp_serde::to_vec(&asset).expect("test wire encodes");

    let error = decode_compiled_dialogue_messagepack(&bytes).expect_err("short digest is rejected");

    assert!(matches!(
        error,
        CompiledAssetDecodeError::MalformedAsset(message) if message.contains("blake3 fingerprint digest must be 32 bytes")
    ));
}

#[test]
fn decode_rejects_trailing_messagepack_bytes() {
    let mut bytes = rmp_serde::to_vec(&valid_wire_asset()).expect("test wire encodes");
    bytes.extend_from_slice(&[0xc0]);

    let error = decode_compiled_dialogue_messagepack(&bytes).expect_err("trailing bytes rejected");

    assert!(matches!(
        error,
        CompiledAssetDecodeError::MalformedAsset(message) if message.contains("trailing bytes")
    ));
}

#[test]
fn decode_rejects_unsorted_or_duplicate_lookup_entries() {
    let mut unsorted = valid_wire_asset();
    unsorted.blocks.push(WireBlock {
        id: "alpha",
        source_file: 0,
        statements: WireRange(0, 0),
        metadata: WireRange(0, 0),
        default_speaker: None,
        source_map: 0,
    });
    unsorted.block_lookup.push(WireLookupEntry {
        id: "alpha",
        index: 1,
    });
    let unsorted_bytes = rmp_serde::to_vec(&unsorted).expect("test wire encodes");

    let unsorted_error = decode_compiled_dialogue_messagepack(&unsorted_bytes)
        .expect_err("unsorted lookup rejected");

    assert!(matches!(
        unsorted_error,
        CompiledAssetDecodeError::MalformedAsset(message) if message.contains("strictly sorted and unique")
    ));

    let mut duplicate = valid_wire_asset();
    duplicate.block_lookup.push(WireLookupEntry {
        id: "start",
        index: 0,
    });
    let duplicate_bytes = rmp_serde::to_vec(&duplicate).expect("test wire encodes");

    let duplicate_error = decode_compiled_dialogue_messagepack(&duplicate_bytes)
        .expect_err("duplicate lookup rejected");

    assert!(matches!(
        duplicate_error,
        CompiledAssetDecodeError::MalformedAsset(message) if message.contains("strictly sorted and unique")
    ));

    let mut alias = valid_wire_asset();
    alias.block_lookup.push(WireLookupEntry {
        id: "zzz_alias",
        index: 0,
    });
    let alias_bytes = rmp_serde::to_vec(&alias).expect("test wire encodes");

    let alias_error =
        decode_compiled_dialogue_messagepack(&alias_bytes).expect_err("lookup alias rejected");

    assert!(matches!(
        alias_error,
        CompiledAssetDecodeError::MalformedAsset(message) if message.contains("entries for 1 table rows")
    ));
}

#[test]
fn decode_rejects_choice_echo_referencing_unknown_line() {
    let mut asset = valid_wire_asset();
    asset.lines.push(WireLine {
        id: "known_line",
        source_text: "Known.",
        speaker: None,
        metadata: WireRange(0, 0),
        source_map: 0,
    });
    asset.line_lookup.push(WireLookupEntry {
        id: "known_line",
        index: 0,
    });
    asset.choices.push(WireChoice {
        id: "ask",
        source_text: "Ask?",
        metadata: WireRange(0, 0),
        condition: None,
        target: Tagged::nil(recite_core::V0_DIVERT_TARGET_TAG_END),
        echo: Tagged::payload(
            recite_core::V0_CHOICE_ECHO_TAG_EXPLICIT_LINE,
            "missing_line",
        ),
        source_map: 0,
    });
    asset.choice_lookup.push(WireLookupEntry {
        id: "ask",
        index: 0,
    });
    let bytes = rmp_serde::to_vec(&asset).expect("test wire encodes");

    let error =
        decode_compiled_dialogue_messagepack(&bytes).expect_err("unknown echo line rejected");

    assert!(matches!(
        error,
        CompiledAssetDecodeError::MalformedAsset(message)
            if message.contains("choice echo")
                && message.contains("missing_line")
    ));
}

#[test]
fn decode_rejects_source_map_file_mismatch() {
    let mut asset = valid_wire_asset();
    asset.source_maps[0].span.file = "dialogue/other.recite";
    let bytes = rmp_serde::to_vec(&asset).expect("test wire encodes");

    let error =
        decode_compiled_dialogue_messagepack(&bytes).expect_err("source map mismatch rejected");

    assert!(matches!(
        error,
        CompiledAssetDecodeError::MalformedAsset(message)
            if message.contains("source map")
                && message.contains("dialogue/other.recite")
                && message.contains("dialogue/main.recite")
    ));
}

#[test]
fn decode_rejects_source_span_end_before_start() {
    let mut asset = valid_wire_asset();
    asset.source_maps[0].span.start_line = 2;
    asset.source_maps[0].span.start_column = 1;
    asset.source_maps[0].span.end_line = Some(1);
    asset.source_maps[0].span.end_column = Some(1);
    let bytes = rmp_serde::to_vec(&asset).expect("test wire encodes");

    let error =
        decode_compiled_dialogue_messagepack(&bytes).expect_err("reversed source span rejected");

    assert!(matches!(
        error,
        CompiledAssetDecodeError::MalformedAsset(message)
            if message.contains("span end precedes span start")
    ));
}

#[test]
fn decode_rejects_non_finite_float_values() {
    let mut asset = valid_wire_asset();
    asset.metadata.push(WireMetadataEntry {
        key: "score",
        value: Tagged::payload(
            recite_core::V0_VALUE_TAG_SCALAR,
            Tagged::payload(recite_core::V0_SCALAR_TAG_FLOAT, f64::NAN),
        ),
        source_map: None,
    });
    let bytes = rmp_serde::to_vec(&asset).expect("test wire encodes");

    let error =
        decode_compiled_dialogue_messagepack(&bytes).expect_err("non-finite float rejected");

    assert!(matches!(
        error,
        CompiledAssetDecodeError::MalformedAsset(message)
            if message.contains("float scalar")
                && message.contains("finite")
    ));
}

#[test]
fn decode_rejects_invalid_compiled_names() {
    let mut metadata = valid_wire_asset();
    metadata.metadata.push(WireMetadataEntry {
        key: "",
        value: Tagged::payload(
            recite_core::V0_VALUE_TAG_SCALAR,
            Tagged::payload(recite_core::V0_SCALAR_TAG_FLOAT, 1.0),
        ),
        source_map: None,
    });
    assert_malformed_asset_contains(metadata, "metadata key");

    let mut effect = valid_wire_asset();
    effect.effects.push(WireEffect {
        id: "fx",
        mode: Tagged::nil(recite_core::V0_EFFECT_MODE_TAG_DEFERRED),
        function: "bad function",
        args: Vec::new(),
        source_map: 0,
    });
    assert_malformed_asset_contains(effect, "effect function");

    let mut condition = valid_wire_asset();
    condition.choices.push(WireChoice {
        id: "ask",
        source_text: "Ask?",
        metadata: WireRange(0, 0),
        condition: Some(WireConditionExpression::Call(WireConditionCall {
            function: "",
            args: Vec::new(),
        })),
        target: Tagged::nil(recite_core::V0_DIVERT_TARGET_TAG_END),
        echo: Tagged::nil(recite_core::V0_CHOICE_ECHO_TAG_NONE),
        source_map: 0,
    });
    condition.choice_lookup.push(WireLookupEntry {
        id: "ask",
        index: 0,
    });
    assert_malformed_asset_contains(condition, "condition function");

    let mut argument = valid_wire_asset();
    argument.effects.push(WireEffect {
        id: "fx",
        mode: Tagged::nil(recite_core::V0_EFFECT_MODE_TAG_DEFERRED),
        function: "advance_thread",
        args: vec![Tagged::payload(
            recite_core::V0_ARGUMENT_TAG_IDENTIFIER,
            "bad argument",
        )],
        source_map: 0,
    });
    assert_malformed_asset_contains(argument, "argument identifier");
}

#[test]
fn decode_rejects_duplicate_source_paths_and_effect_ids() {
    let mut source = valid_wire_asset();
    source.sources.push(WireSourceFile {
        path: "dialogue/main.recite",
        fingerprint: valid_fingerprint(),
    });
    assert_malformed_asset_contains(source, "source file path");

    let mut effect = valid_wire_asset();
    effect.effects.push(WireEffect {
        id: "fx",
        mode: Tagged::nil(recite_core::V0_EFFECT_MODE_TAG_DEFERRED),
        function: "advance_thread",
        args: Vec::new(),
        source_map: 0,
    });
    effect.effects.push(WireEffect {
        id: "fx",
        mode: Tagged::nil(recite_core::V0_EFFECT_MODE_TAG_IMMEDIATE),
        function: "advance_thread",
        args: Vec::new(),
        source_map: 0,
    });
    assert_malformed_asset_contains(effect, "effect id");
}

#[test]
fn decode_rejects_duplicate_line_and_choice_ids() {
    let mut asset = valid_wire_asset();
    asset.lines.push(WireLine {
        id: "shared_id",
        source_text: "Line.",
        speaker: None,
        metadata: WireRange(0, 0),
        source_map: 0,
    });
    asset.line_lookup.push(WireLookupEntry {
        id: "shared_id",
        index: 0,
    });
    asset.choices.push(WireChoice {
        id: "shared_id",
        source_text: "Choice?",
        metadata: WireRange(0, 0),
        condition: None,
        target: Tagged::nil(recite_core::V0_DIVERT_TARGET_TAG_END),
        echo: Tagged::nil(recite_core::V0_CHOICE_ECHO_TAG_NONE),
        source_map: 0,
    });
    asset.choice_lookup.push(WireLookupEntry {
        id: "shared_id",
        index: 0,
    });

    assert_malformed_asset_contains(asset, "line and choice ids");
}

#[test]
fn decode_rejects_empty_prompt_choices_and_condition_groups() {
    let mut prompt = valid_wire_asset();
    prompt.statements[0].kind = WireStatementKind::Prompt {
        line: None,
        choices: WireRange(0, 0),
    };
    assert_malformed_asset_contains(prompt, "prompt choices");

    let mut and_group = valid_wire_asset();
    and_group.choices.push(WireChoice {
        id: "ask",
        source_text: "Ask?",
        metadata: WireRange(0, 0),
        condition: Some(WireConditionExpression::EmptyAnd),
        target: Tagged::nil(recite_core::V0_DIVERT_TARGET_TAG_END),
        echo: Tagged::nil(recite_core::V0_CHOICE_ECHO_TAG_NONE),
        source_map: 0,
    });
    and_group.choice_lookup.push(WireLookupEntry {
        id: "ask",
        index: 0,
    });
    assert_malformed_asset_contains(and_group, "condition and group");

    let mut or_group = valid_wire_asset();
    or_group.choices.push(WireChoice {
        id: "ask",
        source_text: "Ask?",
        metadata: WireRange(0, 0),
        condition: Some(WireConditionExpression::EmptyOr),
        target: Tagged::nil(recite_core::V0_DIVERT_TARGET_TAG_END),
        echo: Tagged::nil(recite_core::V0_CHOICE_ECHO_TAG_NONE),
        source_map: 0,
    });
    or_group.choice_lookup.push(WireLookupEntry {
        id: "ask",
        index: 0,
    });
    assert_malformed_asset_contains(or_group, "condition or group");
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
fn default_block_index_is_stable_when_default_block_is_not_first() {
    let alpha = CompileInput::new(
        "dialogue/alpha.recite",
        concat!(":: alpha\n", "> alpha_line\n", "  Alpha.\n", "-> END\n",),
    );
    let zed = CompileInput::new(
        "dialogue/zed.recite",
        concat!(":: zed default\n", "> zed_line\n", "  Zed.\n", "-> END\n",),
    );

    let output = compile_inputs([zed, alpha], options())
        .expect("compile succeeds")
        .asset
        .expect("asset emitted");

    assert_eq!(
        output
            .dialogue
            .blocks
            .iter()
            .map(|block| block.id.as_str())
            .collect::<Vec<_>>(),
        ["alpha", "zed"]
    );
    assert_eq!(output.dialogue.default_block.as_u32(), 1);
    assert!(
        output.inspection_json.contains("\"default_block\":1"),
        "inspection JSON should expose the default block index"
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

    let missing_echo_line = compile_inputs(
        [CompileInput::new(
            "dialogue/bad.recite",
            concat!(
                ":: start default\n",
                "? choose echo=line(missing_echo_line)\n",
                "  Choose.\n",
                "  -> END\n",
            ),
        )],
        options(),
    )
    .expect("validation diagnostics are not hard errors");

    assert!(missing_echo_line.asset.is_none());
    assert_eq!(
        missing_echo_line.diagnostics[0].code.as_str(),
        "RECITE_VALIDATE015"
    );

    let non_finite_metadata = compile_inputs(
        [CompileInput::new(
            "dialogue/bad.recite",
            concat!(":: start default\n", "> line score=NaN\n", "  Text.\n",),
        )],
        options(),
    )
    .expect("validation diagnostics are not hard errors");

    assert!(non_finite_metadata.asset.is_none());
    assert_eq!(
        non_finite_metadata.diagnostics[0].code.as_str(),
        "RECITE_VALIDATE016"
    );
}

#[test]
fn compile_with_schema_reports_effect_validation_without_asset() {
    let schema = load_schema_manifest_str(
        "fixtures/schema/valid/generated_manifest.json",
        include_str!("../../../fixtures/schema/valid/generated_manifest.json"),
    )
    .schema
    .expect("valid generated manifest fixture");

    let report = compile_inputs_with_schema(
        [CompileInput::new(
            "dialogue/bad.recite",
            concat!(":: start default\n", "! immediate missing_effect(snap)\n"),
        )],
        options(),
        &schema,
    )
    .expect("validation diagnostics are not hard errors");

    assert!(report.asset.is_none());
    assert_eq!(report.diagnostics[0].code.as_str(), "RECITE_VALIDATE017");
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

fn assert_malformed_asset_contains(asset: WireAsset<'_>, expected: &str) {
    let bytes = rmp_serde::to_vec(&asset).expect("test wire encodes");
    let error = decode_compiled_dialogue_messagepack(&bytes).expect_err("asset is rejected");

    assert!(matches!(
        error,
        CompiledAssetDecodeError::MalformedAsset(message) if message.contains(expected)
    ));
}

fn valid_wire_asset() -> WireAsset<'static> {
    WireAsset {
        header: valid_header(),
        default_block: 0,
        sources: vec![WireSourceFile {
            path: "dialogue/main.recite",
            fingerprint: valid_fingerprint(),
        }],
        blocks: vec![WireBlock {
            id: "start",
            source_file: 0,
            statements: WireRange(0, 1),
            metadata: WireRange(0, 0),
            default_speaker: None,
            source_map: 0,
        }],
        statements: vec![WireStatement {
            kind: WireStatementKind::End,
            source_map: 0,
        }],
        match_arms: Vec::new(),
        lines: Vec::new(),
        choices: Vec::new(),
        speakers: Vec::new(),
        metadata: Vec::new(),
        effects: Vec::new(),
        source_maps: vec![WireSourceMapEntry {
            source_file: 0,
            span: WireSourceSpan {
                file: "dialogue/main.recite",
                start_line: 1,
                start_column: 1,
                end_line: None,
                end_column: None,
            },
        }],
        block_lookup: vec![WireLookupEntry {
            id: "start",
            index: 0,
        }],
        line_lookup: Vec::new(),
        choice_lookup: Vec::new(),
    }
}

fn valid_header() -> WireHeader<'static> {
    WireHeader {
        format_version: COMPILED_ASSET_FORMAT_VERSION_V0,
        compiler_compatibility_version: COMPILER_COMPATIBILITY_VERSION_V0,
        primary_encoding: Tagged::<u8>::nil(recite_core::V0_ASSET_ENCODING_MESSAGEPACK),
        inspection_encoding: Tagged::<u8>::nil(recite_core::V0_INSPECTION_ENCODING_COMPACT_JSON),
        compiler_version: "0.0.1",
        asset_id: "dialogue/main.recitec",
        source_map_id: "dialogue/main.recitec.map",
        schema_fingerprint: Tagged::nil(recite_core::V0_SCHEMA_FINGERPRINT_TAG_NO_SCHEMA),
    }
}

fn valid_fingerprint() -> WireFingerprint<'static> {
    WireFingerprint {
        algorithm: "blake3",
        digest: Bytes::new(&VALID_DIGEST),
    }
}

const VALID_DIGEST: [u8; BLAKE3_DIGEST_LEN] = [7; BLAKE3_DIGEST_LEN];
const SHORT_DIGEST: [u8; 3] = [1, 2, 3];

struct WireAsset<'a> {
    header: WireHeader<'a>,
    default_block: u32,
    sources: Vec<WireSourceFile<'a>>,
    blocks: Vec<WireBlock<'a>>,
    statements: Vec<WireStatement>,
    match_arms: Vec<()>,
    lines: Vec<WireLine<'a>>,
    choices: Vec<WireChoice<'a>>,
    speakers: Vec<WireSpeaker<'a>>,
    metadata: Vec<WireMetadataEntry<'a>>,
    effects: Vec<WireEffect<'a>>,
    source_maps: Vec<WireSourceMapEntry<'a>>,
    block_lookup: Vec<WireLookupEntry<'a>>,
    line_lookup: Vec<WireLookupEntry<'a>>,
    choice_lookup: Vec<WireLookupEntry<'a>>,
}

impl Serialize for WireAsset<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(15)?;
        tuple.serialize_element(&self.header)?;
        tuple.serialize_element(&self.default_block)?;
        tuple.serialize_element(&self.sources)?;
        tuple.serialize_element(&self.blocks)?;
        tuple.serialize_element(&self.statements)?;
        tuple.serialize_element(&self.match_arms)?;
        tuple.serialize_element(&self.lines)?;
        tuple.serialize_element(&self.choices)?;
        tuple.serialize_element(&self.speakers)?;
        tuple.serialize_element(&self.metadata)?;
        tuple.serialize_element(&self.effects)?;
        tuple.serialize_element(&self.source_maps)?;
        tuple.serialize_element(&self.block_lookup)?;
        tuple.serialize_element(&self.line_lookup)?;
        tuple.serialize_element(&self.choice_lookup)?;
        tuple.end()
    }
}

struct WireHeader<'a> {
    format_version: u16,
    compiler_compatibility_version: u16,
    primary_encoding: Tagged<u8>,
    inspection_encoding: Tagged<u8>,
    compiler_version: &'a str,
    asset_id: &'a str,
    source_map_id: &'a str,
    schema_fingerprint: Tagged<WireFingerprint<'a>>,
}

impl Serialize for WireHeader<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(8)?;
        tuple.serialize_element(&self.format_version)?;
        tuple.serialize_element(&self.compiler_compatibility_version)?;
        tuple.serialize_element(&self.primary_encoding)?;
        tuple.serialize_element(&self.inspection_encoding)?;
        tuple.serialize_element(&self.compiler_version)?;
        tuple.serialize_element(&self.asset_id)?;
        tuple.serialize_element(&self.source_map_id)?;
        tuple.serialize_element(&self.schema_fingerprint)?;
        tuple.end()
    }
}

struct Tagged<T> {
    tag: u8,
    payload: Option<T>,
}

impl<T> Tagged<T> {
    fn nil(tag: u8) -> Self {
        Self { tag, payload: None }
    }

    fn payload(tag: u8, payload: T) -> Self {
        Self {
            tag,
            payload: Some(payload),
        }
    }
}

impl<T: Serialize> Serialize for Tagged<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(&self.tag)?;
        tuple.serialize_element(&self.payload)?;
        tuple.end()
    }
}

struct WireFingerprint<'a> {
    algorithm: &'a str,
    digest: &'a Bytes,
}

impl Serialize for WireFingerprint<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(&self.algorithm)?;
        tuple.serialize_element(&self.digest)?;
        tuple.end()
    }
}

struct WireSourceFile<'a> {
    path: &'a str,
    fingerprint: WireFingerprint<'a>,
}

impl Serialize for WireSourceFile<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(&self.path)?;
        tuple.serialize_element(&self.fingerprint)?;
        tuple.end()
    }
}

#[derive(Clone, Copy, Serialize)]
struct WireRange(u32, u32);

struct WireBlock<'a> {
    id: &'a str,
    source_file: u32,
    statements: WireRange,
    metadata: WireRange,
    default_speaker: Option<u32>,
    source_map: u32,
}

impl Serialize for WireBlock<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(6)?;
        tuple.serialize_element(&self.id)?;
        tuple.serialize_element(&self.source_file)?;
        tuple.serialize_element(&self.statements)?;
        tuple.serialize_element(&self.metadata)?;
        tuple.serialize_element(&self.default_speaker)?;
        tuple.serialize_element(&self.source_map)?;
        tuple.end()
    }
}

struct WireStatement {
    kind: WireStatementKind,
    source_map: u32,
}

impl Serialize for WireStatement {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(&self.kind)?;
        tuple.serialize_element(&self.source_map)?;
        tuple.end()
    }
}

enum WireStatementKind {
    End,
    Prompt {
        line: Option<u32>,
        choices: WireRange,
    },
}

impl Serialize for WireStatementKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::End => Tagged::<u8>::nil(recite_core::V0_STATEMENT_TAG_END).serialize(serializer),
            Self::Prompt { line, choices } => {
                Tagged::payload(recite_core::V0_STATEMENT_TAG_PROMPT, (*line, *choices))
                    .serialize(serializer)
            }
        }
    }
}

struct WireLine<'a> {
    id: &'a str,
    source_text: &'a str,
    speaker: Option<u32>,
    metadata: WireRange,
    source_map: u32,
}

impl Serialize for WireLine<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(5)?;
        tuple.serialize_element(&self.id)?;
        tuple.serialize_element(&self.source_text)?;
        tuple.serialize_element(&self.speaker)?;
        tuple.serialize_element(&self.metadata)?;
        tuple.serialize_element(&self.source_map)?;
        tuple.end()
    }
}

struct WireChoice<'a> {
    id: &'a str,
    source_text: &'a str,
    metadata: WireRange,
    condition: Option<WireConditionExpression<'a>>,
    target: Tagged<u32>,
    echo: Tagged<&'a str>,
    source_map: u32,
}

impl Serialize for WireChoice<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(7)?;
        tuple.serialize_element(&self.id)?;
        tuple.serialize_element(&self.source_text)?;
        tuple.serialize_element(&self.metadata)?;
        tuple.serialize_element(&self.condition)?;
        tuple.serialize_element(&self.target)?;
        tuple.serialize_element(&self.echo)?;
        tuple.serialize_element(&self.source_map)?;
        tuple.end()
    }
}

enum WireConditionExpression<'a> {
    Call(WireConditionCall<'a>),
    EmptyAnd,
    EmptyOr,
}

impl Serialize for WireConditionExpression<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Call(call) => {
                Tagged::payload(recite_core::V0_CONDITION_TAG_CALL, call).serialize(serializer)
            }
            Self::EmptyAnd => {
                Tagged::payload(recite_core::V0_CONDITION_TAG_AND, Vec::<Self>::new())
                    .serialize(serializer)
            }
            Self::EmptyOr => Tagged::payload(recite_core::V0_CONDITION_TAG_OR, Vec::<Self>::new())
                .serialize(serializer),
        }
    }
}

struct WireConditionCall<'a> {
    function: &'a str,
    args: Vec<Tagged<&'a str>>,
}

impl Serialize for WireConditionCall<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(&self.function)?;
        tuple.serialize_element(&self.args)?;
        tuple.end()
    }
}

struct WireSpeaker<'a> {
    id: &'a str,
}

impl Serialize for WireSpeaker<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(1)?;
        tuple.serialize_element(&self.id)?;
        tuple.end()
    }
}

struct WireMetadataEntry<'a> {
    key: &'a str,
    value: Tagged<Tagged<f64>>,
    source_map: Option<u32>,
}

impl Serialize for WireMetadataEntry<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(3)?;
        tuple.serialize_element(&self.key)?;
        tuple.serialize_element(&self.value)?;
        tuple.serialize_element(&self.source_map)?;
        tuple.end()
    }
}

struct WireEffect<'a> {
    id: &'a str,
    mode: Tagged<u8>,
    function: &'a str,
    args: Vec<Tagged<&'a str>>,
    source_map: u32,
}

impl Serialize for WireEffect<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(5)?;
        tuple.serialize_element(&self.id)?;
        tuple.serialize_element(&self.mode)?;
        tuple.serialize_element(&self.function)?;
        tuple.serialize_element(&self.args)?;
        tuple.serialize_element(&self.source_map)?;
        tuple.end()
    }
}

struct WireSourceMapEntry<'a> {
    source_file: u32,
    span: WireSourceSpan<'a>,
}

impl Serialize for WireSourceMapEntry<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(&self.source_file)?;
        tuple.serialize_element(&self.span)?;
        tuple.end()
    }
}

struct WireSourceSpan<'a> {
    file: &'a str,
    start_line: u32,
    start_column: u32,
    end_line: Option<u32>,
    end_column: Option<u32>,
}

impl Serialize for WireSourceSpan<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(5)?;
        tuple.serialize_element(&self.file)?;
        tuple.serialize_element(&self.start_line)?;
        tuple.serialize_element(&self.start_column)?;
        tuple.serialize_element(&self.end_line)?;
        tuple.serialize_element(&self.end_column)?;
        tuple.end()
    }
}

struct WireLookupEntry<'a> {
    id: &'a str,
    index: u32,
}

impl Serialize for WireLookupEntry<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(&self.id)?;
        tuple.serialize_element(&self.index)?;
        tuple.end()
    }
}
