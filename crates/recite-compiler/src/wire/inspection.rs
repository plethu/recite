use recite_core::{
    ChoiceIndex, CompiledArgument, CompiledAssetEncoding, CompiledChoiceEcho,
    CompiledConditionCall, CompiledConditionExpression, CompiledDialogue, CompiledDivertTarget,
    CompiledEffectMode, CompiledInspectionEncoding, CompiledMatchPattern, CompiledStatementKind,
    ContentFingerprint, LineIndex, MatchArmIndex, MetadataIndex, ScalarValue, SchemaFingerprint,
    SourceMapIndex, SourceSpan, SpeakerIndex, StatementIndex, TableRange, Value,
};
use serde_json::{Value as JsonValue, json};

use super::shared::{hex_lower, range_to_u32};
use crate::compile::CompileError;

pub(crate) fn serialize_inspection_json(
    dialogue: &CompiledDialogue,
) -> Result<String, CompileError> {
    serde_json::to_string(&json_dialogue(dialogue)).map_err(|error| {
        CompileError::Serialization(format!("failed to encode inspection JSON: {error}"))
    })
}

fn json_dialogue(dialogue: &CompiledDialogue) -> JsonValue {
    json!({
        "header": json_header(dialogue),
        "sources": dialogue.sources.iter().map(|source| json!({
            "path": source.path.as_str(),
            "fingerprint": json_fingerprint(&source.fingerprint),
        })).collect::<Vec<_>>(),
        "blocks": dialogue.blocks.iter().map(|block| json!({
            "id": block.id.as_str(),
            "source_file": block.source_file.as_u32(),
            "statements": json_range(block.statements, StatementIndex::as_u32),
            "metadata": json_range(block.metadata, MetadataIndex::as_u32),
            "default_speaker": block.default_speaker.map(SpeakerIndex::as_u32),
            "source_map": block.source_map.as_u32(),
        })).collect::<Vec<_>>(),
        "statements": dialogue.statements.iter().map(|statement| json!({
            "kind": json_statement_kind(&statement.kind),
            "source_map": statement.source_map.as_u32(),
        })).collect::<Vec<_>>(),
        "match_arms": dialogue.match_arms.iter().map(|arm| json!({
            "pattern": json_match_pattern(&arm.pattern),
            "statements": json_range(arm.statements, StatementIndex::as_u32),
            "source_map": arm.source_map.as_u32(),
        })).collect::<Vec<_>>(),
        "lines": dialogue.lines.iter().map(|line| json!({
            "id": line.id.as_str(),
            "source_text": line.source_text.as_str(),
            "speaker": line.speaker.map(SpeakerIndex::as_u32),
            "metadata": json_range(line.metadata, MetadataIndex::as_u32),
            "source_map": line.source_map.as_u32(),
        })).collect::<Vec<_>>(),
        "choices": dialogue.choices.iter().map(|choice| json!({
            "id": choice.id.as_str(),
            "source_text": choice.source_text.as_str(),
            "metadata": json_range(choice.metadata, MetadataIndex::as_u32),
            "condition": choice.condition.as_ref().map(json_condition_expression),
            "target": json_divert_target(&choice.target),
            "echo": json_choice_echo(&choice.echo),
            "source_map": choice.source_map.as_u32(),
        })).collect::<Vec<_>>(),
        "speakers": dialogue.speakers.iter().map(|speaker| json!({
            "id": speaker.id.as_str(),
        })).collect::<Vec<_>>(),
        "metadata": dialogue.metadata.iter().map(|entry| json!({
            "key": entry.key.as_str(),
            "value": json_value(&entry.value),
            "source_map": entry.source_map.map(SourceMapIndex::as_u32),
        })).collect::<Vec<_>>(),
        "effects": dialogue.effects.iter().map(|effect| json!({
            "id": effect.id.as_str(),
            "mode": json_effect_mode(effect.mode),
            "function": effect.function.as_str(),
            "args": effect.args.iter().map(json_argument).collect::<Vec<_>>(),
            "source_map": effect.source_map.as_u32(),
        })).collect::<Vec<_>>(),
        "source_maps": dialogue.source_maps.iter().map(|entry| json!({
            "source_file": entry.source_file.as_u32(),
            "span": json_source_span(&entry.span),
        })).collect::<Vec<_>>(),
        "block_lookup": dialogue.block_lookup.iter().map(|entry| json!({
            "id": entry.id.as_str(),
            "index": entry.index.as_u32(),
        })).collect::<Vec<_>>(),
        "line_lookup": dialogue.line_lookup.iter().map(|entry| json!({
            "id": entry.id.as_str(),
            "index": entry.index.as_u32(),
        })).collect::<Vec<_>>(),
        "choice_lookup": dialogue.choice_lookup.iter().map(|entry| json!({
            "id": entry.id.as_str(),
            "index": entry.index.as_u32(),
        })).collect::<Vec<_>>(),
    })
}

fn json_header(dialogue: &CompiledDialogue) -> JsonValue {
    let header = &dialogue.header;
    json!({
        "format_version": header.format_version,
        "compiler_compatibility_version": header.compiler_compatibility_version,
        "primary_encoding": json_asset_encoding(header.primary_encoding),
        "inspection_encoding": json_inspection_encoding(header.inspection_encoding),
        "compiler_version": header.compiler_version.as_str(),
        "asset_id": header.asset_id.as_str(),
        "source_map_id": header.source_map_id.as_str(),
        "schema_fingerprint": json_schema_fingerprint(&header.schema_fingerprint),
    })
}

fn json_asset_encoding(encoding: CompiledAssetEncoding) -> JsonValue {
    match encoding {
        CompiledAssetEncoding::MessagePack => tagged_json("messagepack", JsonValue::Null),
    }
}

fn json_inspection_encoding(encoding: CompiledInspectionEncoding) -> JsonValue {
    match encoding {
        CompiledInspectionEncoding::CompactJson => tagged_json("compact_json", JsonValue::Null),
    }
}

fn json_schema_fingerprint(fingerprint: &SchemaFingerprint) -> JsonValue {
    match fingerprint {
        SchemaFingerprint::Fingerprint(fingerprint) => {
            tagged_json("fingerprint", json_fingerprint(fingerprint))
        }
        SchemaFingerprint::NoSchema => tagged_json("no_schema", JsonValue::Null),
    }
}

fn json_fingerprint(fingerprint: &ContentFingerprint) -> JsonValue {
    json!({
        "algorithm": fingerprint.algorithm().as_str(),
        "digest": hex_lower(fingerprint.digest().as_bytes()),
    })
}

fn json_statement_kind(kind: &CompiledStatementKind) -> JsonValue {
    match kind {
        CompiledStatementKind::Line(index) => tagged_json("line", json!(index.as_u32())),
        CompiledStatementKind::Prompt { line, choices } => tagged_json(
            "prompt",
            json!({
                "line": line.map(LineIndex::as_u32),
                "choices": json_range(*choices, ChoiceIndex::as_u32),
            }),
        ),
        CompiledStatementKind::Divert(target) => tagged_json("divert", json_divert_target(target)),
        CompiledStatementKind::If {
            condition,
            then_statements,
            else_statements,
        } => tagged_json(
            "if",
            json!({
                "condition": json_condition_expression(condition),
                "then_statements": json_range(*then_statements, StatementIndex::as_u32),
                "else_statements": json_range(*else_statements, StatementIndex::as_u32),
            }),
        ),
        CompiledStatementKind::Match { scrutinee, arms } => tagged_json(
            "match",
            json!({
                "scrutinee": json_condition_call(scrutinee),
                "arms": json_range(*arms, MatchArmIndex::as_u32),
            }),
        ),
        CompiledStatementKind::Effect(index) => tagged_json("effect", json!(index.as_u32())),
        CompiledStatementKind::End => tagged_json("end", JsonValue::Null),
    }
}

fn json_match_pattern(pattern: &CompiledMatchPattern) -> JsonValue {
    match pattern {
        CompiledMatchPattern::Variant(value) => tagged_json("variant", json!(value)),
        CompiledMatchPattern::Wildcard => tagged_json("wildcard", JsonValue::Null),
    }
}

fn json_divert_target(target: &CompiledDivertTarget) -> JsonValue {
    match target {
        CompiledDivertTarget::Block(index) => tagged_json("block", json!(index.as_u32())),
        CompiledDivertTarget::End => tagged_json("end", JsonValue::Null),
    }
}

fn json_choice_echo(echo: &CompiledChoiceEcho) -> JsonValue {
    match echo {
        CompiledChoiceEcho::None => tagged_json("none", JsonValue::Null),
        CompiledChoiceEcho::SelectedText => tagged_json("selected_text", JsonValue::Null),
        CompiledChoiceEcho::ExplicitLine(line_id) => {
            tagged_json("explicit_line", json!(line_id.as_str()))
        }
    }
}

fn json_effect_mode(mode: CompiledEffectMode) -> JsonValue {
    match mode {
        CompiledEffectMode::Deferred => tagged_json("deferred", JsonValue::Null),
        CompiledEffectMode::Immediate => tagged_json("immediate", JsonValue::Null),
        CompiledEffectMode::Blocking => tagged_json("blocking", JsonValue::Null),
    }
}

fn json_condition_expression(condition: &CompiledConditionExpression) -> JsonValue {
    match condition {
        CompiledConditionExpression::Call(call) => tagged_json("call", json_condition_call(call)),
        CompiledConditionExpression::And(expressions) => tagged_json(
            "and",
            json!(
                expressions
                    .iter()
                    .map(json_condition_expression)
                    .collect::<Vec<_>>()
            ),
        ),
        CompiledConditionExpression::Or(expressions) => tagged_json(
            "or",
            json!(
                expressions
                    .iter()
                    .map(json_condition_expression)
                    .collect::<Vec<_>>()
            ),
        ),
        CompiledConditionExpression::Not(expression) => {
            tagged_json("not", json_condition_expression(expression))
        }
    }
}

fn json_condition_call(call: &CompiledConditionCall) -> JsonValue {
    json!({
        "function": call.function.as_str(),
        "args": call.args.iter().map(json_argument).collect::<Vec<_>>(),
    })
}

fn json_argument(argument: &CompiledArgument) -> JsonValue {
    match argument {
        CompiledArgument::Identifier(value) => tagged_json("identifier", json!(value)),
        CompiledArgument::Value(value) => tagged_json("value", json_scalar_value(value)),
    }
}

fn json_value(value: &Value) -> JsonValue {
    match value {
        Value::Scalar(value) => tagged_json("scalar", json_scalar_value(value)),
        Value::Array(values) => tagged_json(
            "array",
            json!(values.iter().map(json_scalar_value).collect::<Vec<_>>()),
        ),
    }
}

fn json_scalar_value(value: &ScalarValue) -> JsonValue {
    match value {
        ScalarValue::String(value) => tagged_json("string", json!(value)),
        ScalarValue::Integer(value) => tagged_json("integer", json!(value)),
        ScalarValue::Float(value) => tagged_json("float", json!(value)),
        ScalarValue::Boolean(value) => tagged_json("boolean", json!(value)),
    }
}

fn json_source_span(span: &SourceSpan) -> JsonValue {
    json!({
        "file": span.file.as_str(),
        "start_line": span.start.line(),
        "start_column": span.start.column(),
        "end_line": span.end.map(|position| position.line()),
        "end_column": span.end.map(|position| position.column()),
    })
}

fn json_range<I: Copy>(range: TableRange<I>, index: impl Fn(I) -> u32) -> JsonValue {
    let (start, len) = range_to_u32(range, index);
    json!({ "start": start, "len": len })
}

fn tagged_json(tag: &'static str, payload: JsonValue) -> JsonValue {
    json!({ "tag": tag, "payload": payload })
}
