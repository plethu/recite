use recite_compiler::{CompileInput, CompileOptions, compile_inputs, compile_inputs_with_schema};
use recite_core::{
    BLAKE3_DIGEST_LEN, CompiledArgument, CompiledAssetEncoding, CompiledAssetId,
    CompiledChoiceEcho, CompiledConditionExpression, CompiledDialogue, CompiledDivertTarget,
    CompiledEffectMode, CompiledInspectionEncoding, CompiledMatchPattern, CompiledStatementKind,
    CompilerVersion, ContentFingerprint, ScalarValue, SchemaFingerprint, SourceMapId, Value,
    decode_compiled_dialogue_messagepack, load_schema_manifest_str,
};
use std::collections::BTreeSet;

use super::options;

#[test]
fn compiler_generated_messagepack_round_trips_the_v0_tag_surface() {
    let schema = load_schema_manifest_str(
        "fixtures/schema/valid/generated_manifest.json",
        include_str!("../../../../fixtures/schema/valid/generated_manifest.json"),
    )
    .schema
    .expect("valid generated manifest fixture");

    let schema_tag_report = compile_inputs_with_schema(
        [CompileInput::new(
            "dialogue/tag-surface.recite",
            concat!(
                ":: start default speaker=hazel\n",
                "> prompt_line portrait=\"neutral\"\n",
                "  Choose.\n",
                "  ? choose_none\n",
                "    No echo.\n",
                "    -> END\n",
                "  ? choose_selected echo=selected_text if trust_gte(hazel, rhea, 1) or not trust_gte(rhea, hazel, 2)\n",
                "    Selected.\n",
                "    -> branch\n",
                "  ? choose_explicit echo=line(echo_line)\n",
                "    Explicit.\n",
                "    -> END\n",
                ":if not trust_gte(hazel, rhea, 3) and trust_gte(rhea, hazel, 1)\n",
                "  > then_line\n",
                "    Then.\n",
                ":else\n",
                "  > else_line\n",
                "    Else.\n",
                "! immediate play_sfx(snap)\n",
                "! deferred advance_thread(hazel_intro, tired)\n",
                "! blocking advance_thread(hazel_intro, completed)\n",
                "! immediate scalar_effect(\"label\", 3, 1.5, true)\n",
                ":match thread_stage(hazel_intro)\n",
                "  :case fresh\n",
                "    > fresh_line\n",
                "      Fresh.\n",
                "  :case tired\n",
                "    > tired_line\n",
                "      Tired.\n",
                "  :case angry\n",
                "    > angry_line\n",
                "      Angry.\n",
                "  :case fine\n",
                "    > fine_line\n",
                "      Fine.\n",
                "  :case _\n",
                "    > fallback_line\n",
                "      Fallback.\n",
                "-> branch\n",
                "\n",
                ":: branch\n",
                "> echo_line\n",
                "  Echo text.\n",
                "-> END\n",
            ),
        )],
        options_with_schema_fingerprint(),
        &schema,
    )
    .expect("schema tag-surface compile does not hard-fail");
    assert!(
        schema_tag_report.diagnostics.is_empty(),
        "tag-surface fixture should compile without diagnostics: {:?}",
        schema_tag_report.diagnostics
    );
    let schema_tag_asset = schema_tag_report
        .asset
        .expect("valid fixture emits an asset");
    let decoded_schema_tag_asset =
        decode_compiled_dialogue_messagepack(&schema_tag_asset.messagepack)
            .expect("compiler-generated MessagePack tag surface decodes");
    assert_eq!(decoded_schema_tag_asset, schema_tag_asset.dialogue);
    assert_schema_tag_surface_is_covered(&decoded_schema_tag_asset);

    let value_tag_report = compile_inputs(
        [CompileInput::new(
            "dialogue/value-tags.recite",
            concat!(
                ":: values default\n",
                "> values_line label=\"plain\" count=2 weight=1.5 active=true tags=[door, \"mug clang\", true, 2, 1.5]\n",
                "  Values.\n",
                "-> END\n",
            ),
        )],
        options(),
    )
    .expect("value tag-surface compile does not hard-fail");
    assert!(
        value_tag_report.diagnostics.is_empty(),
        "value tag-surface fixture should compile without diagnostics: {:?}",
        value_tag_report.diagnostics
    );
    let value_tag_asset = value_tag_report
        .asset
        .expect("valid fixture emits an asset");
    let decoded_value_tag_asset =
        decode_compiled_dialogue_messagepack(&value_tag_asset.messagepack)
            .expect("compiler-generated MessagePack value tags decode");
    assert_eq!(decoded_value_tag_asset, value_tag_asset.dialogue);
    assert_value_tag_surface_is_covered(&decoded_value_tag_asset);
}

fn options_with_schema_fingerprint() -> CompileOptions {
    CompileOptions::new(
        CompilerVersion::new("0.0.1").expect("valid compiler version"),
        CompiledAssetId::new("dialogue/main.recitec").expect("valid asset id"),
        SourceMapId::new("dialogue/main.recitec.map").expect("valid source map id"),
        SchemaFingerprint::Fingerprint(
            ContentFingerprint::blake3(vec![0x42; BLAKE3_DIGEST_LEN])
                .expect("valid schema fingerprint"),
        ),
    )
}

fn assert_schema_tag_surface_is_covered(dialogue: &CompiledDialogue) {
    assert!(matches!(
        dialogue.header.primary_encoding,
        CompiledAssetEncoding::MessagePack
    ));
    assert!(matches!(
        dialogue.header.inspection_encoding,
        CompiledInspectionEncoding::CompactJson
    ));
    assert!(matches!(
        dialogue.header.schema_fingerprint,
        SchemaFingerprint::Fingerprint(_)
    ));

    let mut statement_tags = BTreeSet::new();
    let mut divert_tags = BTreeSet::new();
    let mut condition_tags = BTreeSet::new();
    let mut argument_tags = BTreeSet::new();
    let mut scalar_tags = BTreeSet::new();
    for statement in &dialogue.statements {
        collect_statement_tag(
            &statement.kind,
            &mut statement_tags,
            &mut divert_tags,
            &mut condition_tags,
            &mut argument_tags,
            &mut scalar_tags,
        );
    }
    for choice in &dialogue.choices {
        collect_divert_tag(&choice.target, &mut divert_tags);
        if let Some(condition) = &choice.condition {
            collect_condition_tags(
                condition,
                &mut condition_tags,
                &mut argument_tags,
                &mut scalar_tags,
            );
        }
    }
    for effect in &dialogue.effects {
        for argument in &effect.args {
            collect_argument_tags(argument, &mut argument_tags, &mut scalar_tags);
        }
    }

    assert_eq!(
        statement_tags,
        BTreeSet::from(["divert", "effect", "end", "if", "line", "match", "prompt"])
    );
    assert_eq!(divert_tags, BTreeSet::from(["block", "end"]));
    assert_eq!(condition_tags, BTreeSet::from(["and", "call", "not", "or"]));
    assert_eq!(argument_tags, BTreeSet::from(["identifier", "value"]));
    assert_eq!(
        scalar_tags,
        BTreeSet::from(["boolean", "float", "integer", "string"])
    );

    let choice_echo_tags = dialogue
        .choices
        .iter()
        .map(|choice| choice_echo_tag(&choice.echo))
        .collect::<BTreeSet<_>>();
    assert_eq!(
        choice_echo_tags,
        BTreeSet::from(["explicit_line", "none", "selected_text"])
    );

    let effect_mode_tags = dialogue
        .effects
        .iter()
        .map(|effect| effect_mode_tag(effect.mode))
        .collect::<BTreeSet<_>>();
    assert_eq!(
        effect_mode_tags,
        BTreeSet::from(["blocking", "deferred", "immediate"])
    );

    let match_pattern_tags = dialogue
        .match_arms
        .iter()
        .map(|arm| match_pattern_tag(&arm.pattern))
        .collect::<BTreeSet<_>>();
    assert_eq!(match_pattern_tags, BTreeSet::from(["variant", "wildcard"]));
}

fn assert_value_tag_surface_is_covered(dialogue: &CompiledDialogue) {
    assert!(matches!(
        dialogue.header.schema_fingerprint,
        SchemaFingerprint::NoSchema
    ));

    let mut value_tags = BTreeSet::new();
    let mut scalar_tags = BTreeSet::new();
    for metadata in &dialogue.metadata {
        collect_value_tags(&metadata.value, &mut value_tags, &mut scalar_tags);
    }

    assert_eq!(value_tags, BTreeSet::from(["array", "scalar"]));
    assert_eq!(
        scalar_tags,
        BTreeSet::from(["boolean", "float", "integer", "string"])
    );

    assert_eq!(
        dialogue
            .metadata
            .iter()
            .find(|entry| entry.key == "label")
            .map(|entry| &entry.value),
        Some(&Value::Scalar(ScalarValue::String("plain".to_owned())))
    );
    assert_eq!(
        dialogue
            .metadata
            .iter()
            .find(|entry| entry.key == "tags")
            .map(|entry| &entry.value),
        Some(&Value::Array(vec![
            ScalarValue::String("door".to_owned()),
            ScalarValue::String("mug clang".to_owned()),
            ScalarValue::Boolean(true),
            ScalarValue::Integer(2),
            ScalarValue::Float(1.5),
        ]))
    );
}

fn collect_statement_tag(
    kind: &CompiledStatementKind,
    statement_tags: &mut BTreeSet<&'static str>,
    divert_tags: &mut BTreeSet<&'static str>,
    condition_tags: &mut BTreeSet<&'static str>,
    argument_tags: &mut BTreeSet<&'static str>,
    scalar_tags: &mut BTreeSet<&'static str>,
) {
    match kind {
        CompiledStatementKind::Line(_) => {
            statement_tags.insert("line");
        }
        CompiledStatementKind::Prompt { .. } => {
            statement_tags.insert("prompt");
        }
        CompiledStatementKind::Divert(target) => {
            statement_tags.insert("divert");
            collect_divert_tag(target, divert_tags);
        }
        CompiledStatementKind::If { condition, .. } => {
            statement_tags.insert("if");
            collect_condition_tags(condition, condition_tags, argument_tags, scalar_tags);
        }
        CompiledStatementKind::Match { scrutinee, .. } => {
            statement_tags.insert("match");
            for argument in &scrutinee.args {
                collect_argument_tags(argument, argument_tags, scalar_tags);
            }
        }
        CompiledStatementKind::Effect(_) => {
            statement_tags.insert("effect");
        }
        CompiledStatementKind::End => {
            statement_tags.insert("end");
        }
    }
}

fn collect_divert_tag(target: &CompiledDivertTarget, tags: &mut BTreeSet<&'static str>) {
    match target {
        CompiledDivertTarget::Block(_) => {
            tags.insert("block");
        }
        CompiledDivertTarget::End => {
            tags.insert("end");
        }
    }
}

fn collect_condition_tags(
    condition: &CompiledConditionExpression,
    condition_tags: &mut BTreeSet<&'static str>,
    argument_tags: &mut BTreeSet<&'static str>,
    scalar_tags: &mut BTreeSet<&'static str>,
) {
    match condition {
        CompiledConditionExpression::Call(call) => {
            condition_tags.insert("call");
            for argument in &call.args {
                collect_argument_tags(argument, argument_tags, scalar_tags);
            }
        }
        CompiledConditionExpression::And(expressions) => {
            condition_tags.insert("and");
            for expression in expressions {
                collect_condition_tags(expression, condition_tags, argument_tags, scalar_tags);
            }
        }
        CompiledConditionExpression::Or(expressions) => {
            condition_tags.insert("or");
            for expression in expressions {
                collect_condition_tags(expression, condition_tags, argument_tags, scalar_tags);
            }
        }
        CompiledConditionExpression::Not(expression) => {
            condition_tags.insert("not");
            collect_condition_tags(expression, condition_tags, argument_tags, scalar_tags);
        }
    }
}

fn collect_argument_tags(
    argument: &CompiledArgument,
    argument_tags: &mut BTreeSet<&'static str>,
    scalar_tags: &mut BTreeSet<&'static str>,
) {
    match argument {
        CompiledArgument::Identifier(_) => {
            argument_tags.insert("identifier");
        }
        CompiledArgument::Value(value) => {
            argument_tags.insert("value");
            scalar_tags.insert(scalar_tag(value));
        }
    }
}

fn collect_value_tags(
    value: &Value,
    value_tags: &mut BTreeSet<&'static str>,
    scalar_tags: &mut BTreeSet<&'static str>,
) {
    match value {
        Value::Scalar(value) => {
            value_tags.insert("scalar");
            scalar_tags.insert(scalar_tag(value));
        }
        Value::Array(values) => {
            value_tags.insert("array");
            for value in values {
                scalar_tags.insert(scalar_tag(value));
            }
        }
    }
}

fn choice_echo_tag(echo: &CompiledChoiceEcho) -> &'static str {
    match echo {
        CompiledChoiceEcho::None => "none",
        CompiledChoiceEcho::SelectedText => "selected_text",
        CompiledChoiceEcho::ExplicitLine(_) => "explicit_line",
    }
}

fn effect_mode_tag(mode: CompiledEffectMode) -> &'static str {
    match mode {
        CompiledEffectMode::Deferred => "deferred",
        CompiledEffectMode::Immediate => "immediate",
        CompiledEffectMode::Blocking => "blocking",
    }
}

fn match_pattern_tag(pattern: &CompiledMatchPattern) -> &'static str {
    match pattern {
        CompiledMatchPattern::Variant(_) => "variant",
        CompiledMatchPattern::Wildcard => "wildcard",
    }
}

fn scalar_tag(value: &ScalarValue) -> &'static str {
    match value {
        ScalarValue::String(_) => "string",
        ScalarValue::Integer(_) => "integer",
        ScalarValue::Float(_) => "float",
        ScalarValue::Boolean(_) => "boolean",
    }
}
