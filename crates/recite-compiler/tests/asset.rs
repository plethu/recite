#![cfg(test)]

use recite_compiler::{
    CompileInput, CompileOptions, CompiledAssetOutput, compile_inputs, compile_inputs_with_schema,
};
use recite_core::{
    AvailabilityReasonArgBinding, AvailabilityReasonDefinition, AvailabilityReasonId,
    BLAKE3_DIGEST_LEN, COMPILED_ASSET_FORMAT_VERSION_V0, COMPILER_COMPATIBILITY_VERSION_V0,
    CompiledAssetEncoding, CompiledAssetId, CompiledAvailabilityReasonArgValue, CompiledEffectMode,
    CompiledInspectionEncoding, CompiledStatementKind, CompilerVersion,
    ConditionAvailabilityReasonMapping, ConditionDefinition, ConditionReturnType,
    ParameterDefinition, ProjectSchema, SchemaFingerprint, SchemaLiteralValue, SchemaTypeRef,
    SourceMapId, StatementIndex, canonical_source_fingerprint,
    decode_compiled_dialogue_messagepack, load_schema_manifest_str,
};
#[path = "../../../tests/support/fixtures.rs"]
#[allow(dead_code)]
mod fixture_support;

#[path = "asset/tag_surface.rs"]
mod tag_surface;

#[path = "asset/wire_golden.rs"]
mod wire_golden;

#[path = "asset/wire_contract.rs"]
mod wire_contract;

#[path = "asset/shared_pressure.rs"]
mod shared_pressure;

#[path = "asset/fingerprint.rs"]
mod fingerprint;

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
        [
            "2119548317bb586e3865",
            "637b1854a7f3ed42f045",
            "75bfa2976ec9548f608a"
        ]
    );
    assert_eq!(
        dialogue
            .choice_lookup
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        ["b84cc9fa241a33bcdf05"]
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
    assert_eq!(intro.id.as_str(), "637b1854a7f3ed42f045");
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
fn plural_lines_round_trip_both_source_forms_through_wire_and_inspection() {
    let report = compile_inputs(
        [CompileInput::new(
            "dialogue/plural.recite",
            concat!(
                ":: start default\n",
                "> letters@22222222222222222222 bind=(count:int=$remaining)\n",
                "  You have one letter.\n",
                "  | You have {count} letters.\n",
                "-> END\n",
            ),
        )],
        options(),
    )
    .expect("plural source compiles");
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    let asset = report.asset.expect("plural source emits an asset");
    let line = &asset.dialogue.lines[0];
    assert_eq!(line.source_text, "You have one letter.");
    assert_eq!(
        line.plural_source_text.as_deref(),
        Some("You have {count} letters.")
    );
    assert_eq!(
        line.authored_plural_source_text.as_deref(),
        Some("You have {count} letters.")
    );

    let decoded = decode_compiled_dialogue_messagepack(&asset.messagepack)
        .expect("plural asset decodes through the v0 wire mirror");
    assert_eq!(decoded.lines, asset.dialogue.lines);
    let inspection: serde_json::Value =
        serde_json::from_str(&asset.inspection_json).expect("inspection JSON parses");
    assert_eq!(
        inspection["lines"][0]["plural_source_text"],
        "You have {count} letters."
    );
    assert_eq!(
        inspection["lines"][0]["authored_plural_source_text"],
        "You have {count} letters."
    );
}

#[test]
fn schema_availability_reasons_compile_into_runtime_asset() {
    let schema = load_schema_manifest_str(
        "fixtures/schema/valid/generated_manifest.json",
        include_str!("../../../fixtures/schema/valid/generated_manifest.json"),
    )
    .schema
    .expect("valid generated manifest fixture");
    let report = compile_inputs_with_schema(
        [CompileInput::new(
            "dialogue/start.recite",
            concat!(
                ":: start default\n",
                "? ask_news@a94b83f2d8bd65101cc3 requires=(trust_gte(hazel, rhea, 3)) reason=innkeeper_trust_hint\n",
                "  What's the news?\n",
                "  -> END\n",
            ),
        )],
        CompileOptions::new(
            CompilerVersion::new("0.0.1").expect("valid compiler version"),
            CompiledAssetId::new("dialogue/main.recitec").expect("valid asset id"),
            SourceMapId::new("dialogue/main.recitec.map").expect("valid source map id"),
            schema.canonical_fingerprint(),
        ),
        &schema,
    )
    .expect("compile succeeds");
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    let asset = report.asset.expect("asset emitted");

    assert_eq!(
        asset.dialogue.choices[0]
            .availability_requirement_source_text
            .as_deref(),
        Some("requires=(trust_gte(hazel, rhea, 3))")
    );
    assert_eq!(asset.dialogue.availability_reasons.len(), 2);
    assert_eq!(
        asset.dialogue.condition_availability_reasons[0].function,
        "trust_gte"
    );

    let inspection: serde_json::Value =
        serde_json::from_str(&asset.inspection_json).expect("inspection JSON parses");
    assert_eq!(
        inspection["choices"][0]["availability_reason_override"],
        "innkeeper_trust_hint"
    );
    assert_eq!(
        inspection["condition_availability_reasons"][0]["args"][2]["name"],
        "threshold"
    );
    assert_eq!(
        inspection["condition_availability_reasons"][0]["args"][2]["value"]["tag"],
        "condition_arg"
    );
    assert_eq!(
        inspection["condition_availability_reasons"][0]["args"][2]["value"]["payload"],
        2
    );
    assert_eq!(
        asset.dialogue.condition_availability_reasons[0].args[2].value,
        CompiledAvailabilityReasonArgValue::ConditionArg(2)
    );
}

#[test]
fn compiler_consumes_full_manifest_without_mixing_producer_metadata_into_schema_identity() {
    let schema = load_schema_manifest_str(
        "fixtures/schema/valid/full_manifest.json",
        include_str!("../../../fixtures/schema/valid/full_manifest.json"),
    )
    .schema
    .expect("full manifest fixture is valid");
    let report = compile_inputs_with_schema(
        [CompileInput::new(
            "dialogue/start.recite",
            ":: start default\n> intro@637b1854a7f3ed42f045\n  Hello.\n-> END\n",
        )],
        CompileOptions::new(
            CompilerVersion::new("0.0.1").expect("valid compiler version"),
            CompiledAssetId::new("dialogue/main.recitec").expect("valid asset id"),
            SourceMapId::new("dialogue/main.recitec.map").expect("valid source map id"),
            schema.canonical_fingerprint(),
        ),
        &schema,
    )
    .expect("compile succeeds");
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    assert!(
        report.asset.is_some(),
        "producer metadata must not block compile"
    );
}

#[test]
fn compile_with_programmatic_schema_rejects_non_finite_availability_reason_literals() {
    let mut schema = ProjectSchema::empty_v1();
    let reason_id = AvailabilityReasonId::new("blocked_reason").expect("valid reason id");
    schema.availability_reasons.insert(
        reason_id.clone(),
        AvailabilityReasonDefinition {
            template: "Need {threshold}".to_owned(),
            params: vec![ParameterDefinition {
                name: "threshold".to_owned(),
                type_ref: SchemaTypeRef::Float,
            }],
            origin: None,
        },
    );
    schema.conditions.insert(
        "blocked".to_owned(),
        ConditionDefinition {
            params: Vec::new(),
            returns: ConditionReturnType::Bool,
            availability_reason: Some(ConditionAvailabilityReasonMapping {
                reason: reason_id,
                args: [(
                    "threshold".to_owned(),
                    AvailabilityReasonArgBinding::Literal(SchemaLiteralValue::Float(
                        "NaN".to_owned(),
                    )),
                )]
                .into(),
            }),
        },
    );

    let error = compile_inputs_with_schema(
        [CompileInput::new(
            "dialogue/start.recite",
            concat!(
                ":: start default\n",
                "? blocked_choice@fa18ce77aacd995c1666 requires=(blocked())\n",
                "  Blocked.\n",
                "  -> END\n",
            ),
        )],
        CompileOptions::new(
            CompilerVersion::new("0.0.1").expect("valid compiler version"),
            CompiledAssetId::new("dialogue/main.recitec").expect("valid asset id"),
            SourceMapId::new("dialogue/main.recitec.map").expect("valid source map id"),
            schema.canonical_fingerprint(),
        ),
        &schema,
    )
    .expect_err("non-finite programmatic availability literals hard-fail compile");

    assert!(
        error
            .to_string()
            .contains("availability reason float literal is not finite"),
        "{error}"
    );
}

#[test]
fn compilation_output_is_deterministic_for_identical_inputs() {
    let first = compile_fixture("fixtures/recite/valid/core_language_spike.recite");
    let second = compile_fixture("fixtures/recite/valid/core_language_spike.recite");

    assert_eq!(first.messagepack, second.messagepack);
    assert_eq!(first.inspection_json, second.inspection_json);

    let decoded =
        decode_compiled_dialogue_messagepack(&first.messagepack).expect("messagepack decodes");
    assert_eq!(decoded.default_block.as_u32(), 0);
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
                "> intro@ddb0434d0a42138a483b ui:portrait=flat\n",
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
fn compilation_order_is_canonical_for_project_inputs() {
    let start = CompileInput::new(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line@3ddabe33285601e01c18\n",
            "  Start.\n",
            "-> dialogue/next.recite::next\n",
        ),
    );
    let next = CompileInput::new(
        "dialogue/next.recite",
        concat!(
            ":: next\n",
            "> next_line@11111111111111111111\n",
            "  Next.\n",
            "-> END\n",
        ),
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
        concat!(
            ":: alpha\n",
            "> alpha_line@11111111111111111111\n",
            "  Alpha.\n",
            "-> END\n",
        ),
    );
    let zed = CompileInput::new(
        "dialogue/zed.recite",
        concat!(
            ":: zed default\n",
            "> zed_line@22222222222222222222\n",
            "  Zed.\n",
            "-> END\n",
        ),
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
                "> line@a543a786de9b562d6229\n",
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
            concat!(
                ":: start default\n",
                "? missing_target@11111111111111111111\n",
                "  Choose.\n",
            ),
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
                "? choose@8913b182d2fb5b7fe6ed echo=line(4b1b2483a1e83884c933)\n",
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
            concat!(
                ":: start default\n",
                "> line@11111111111111111111 score=NaN\n",
                "  Text.\n",
            ),
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
