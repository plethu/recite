//! Tagged control-flow payloads share the borrowed inspection projection.
use recite_core::compiled::{
    ChoiceIndex, CompiledAssetEncoding, CompiledChoiceEcho, CompiledConditionCall,
    CompiledConditionExpression, CompiledDivertTarget, CompiledEffectMode,
    CompiledInspectionEncoding, CompiledMatchPattern, CompiledStatementKind, LineIndex,
    MatchArmIndex, SchemaFingerprint, StatementIndex,
};
use serde::{Serialize, Serializer, ser::SerializeStruct};

use super::{Inspection, Range, Rows, range, serialize_tagged};

impl Serialize for Inspection<'_, CompiledStatementKind> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            CompiledStatementKind::Line(index) => {
                serialize_tagged(serializer, "line", &index.as_u32())
            }
            CompiledStatementKind::Prompt { line, choices } => serialize_tagged(
                serializer,
                "prompt",
                &Prompt {
                    choices: range(*choices, ChoiceIndex::as_u32),
                    line: line.map(LineIndex::as_u32),
                },
            ),
            CompiledStatementKind::Divert(target) => {
                serialize_tagged(serializer, "divert", &Inspection(target))
            }
            CompiledStatementKind::If {
                condition,
                then_statements,
                else_statements,
            } => serialize_tagged(
                serializer,
                "if",
                &If {
                    condition: Inspection(condition),
                    else_statements: range(*else_statements, StatementIndex::as_u32),
                    then_statements: range(*then_statements, StatementIndex::as_u32),
                },
            ),
            CompiledStatementKind::Match { scrutinee, arms } => serialize_tagged(
                serializer,
                "match",
                &Match {
                    arms: range(*arms, MatchArmIndex::as_u32),
                    scrutinee: Inspection(scrutinee),
                },
            ),
            CompiledStatementKind::Effect(index) => {
                serialize_tagged(serializer, "effect", &index.as_u32())
            }
            CompiledStatementKind::End => serialize_tagged(serializer, "end", &()),
        }
    }
}

#[derive(Serialize)]
struct Prompt {
    choices: Range,
    line: Option<u32>,
}

#[derive(Serialize)]
struct If<'a> {
    condition: Inspection<'a, CompiledConditionExpression>,
    else_statements: Range,
    then_statements: Range,
}

#[derive(Serialize)]
struct Match<'a> {
    arms: Range,
    scrutinee: Inspection<'a, CompiledConditionCall>,
}

impl Serialize for Inspection<'_, CompiledConditionExpression> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            CompiledConditionExpression::Call(call) => {
                serialize_tagged(serializer, "call", &Inspection(call))
            }
            CompiledConditionExpression::And(expressions) => {
                serialize_tagged(serializer, "and", &Rows(expressions))
            }
            CompiledConditionExpression::Or(expressions) => {
                serialize_tagged(serializer, "or", &Rows(expressions))
            }
            CompiledConditionExpression::Not(expression) => {
                serialize_tagged(serializer, "not", &Inspection(expression.as_ref()))
            }
        }
    }
}

impl Serialize for Inspection<'_, CompiledConditionCall> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let call = self.0;
        let mut object = serializer.serialize_struct("CompiledConditionCall", 2)?;
        object.serialize_field("args", &Rows(&call.args))?;
        object.serialize_field("function", call.function.as_str())?;
        object.end()
    }
}

impl Serialize for Inspection<'_, CompiledMatchPattern> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            CompiledMatchPattern::Variant(value) => serialize_tagged(serializer, "variant", value),
            CompiledMatchPattern::Wildcard => serialize_tagged(serializer, "wildcard", &()),
        }
    }
}

impl Serialize for Inspection<'_, CompiledDivertTarget> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            CompiledDivertTarget::Block(index) => {
                serialize_tagged(serializer, "block", &index.as_u32())
            }
            CompiledDivertTarget::End => serialize_tagged(serializer, "end", &()),
        }
    }
}

impl Serialize for Inspection<'_, CompiledChoiceEcho> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            CompiledChoiceEcho::None => serialize_tagged(serializer, "none", &()),
            CompiledChoiceEcho::SelectedText => serialize_tagged(serializer, "selected_text", &()),
            CompiledChoiceEcho::ExplicitLine(id) => {
                serialize_tagged(serializer, "explicit_line", id.as_str())
            }
        }
    }
}

impl Serialize for Inspection<'_, CompiledEffectMode> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let tag = match self.0 {
            CompiledEffectMode::Deferred => "deferred",
            CompiledEffectMode::Immediate => "immediate",
            CompiledEffectMode::Blocking => "blocking",
        };
        serialize_tagged(serializer, tag, &())
    }
}

impl Serialize for Inspection<'_, CompiledAssetEncoding> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            CompiledAssetEncoding::MessagePack => serialize_tagged(serializer, "messagepack", &()),
        }
    }
}

impl Serialize for Inspection<'_, CompiledInspectionEncoding> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            CompiledInspectionEncoding::CompactJson => {
                serialize_tagged(serializer, "compact_json", &())
            }
        }
    }
}

impl Serialize for Inspection<'_, SchemaFingerprint> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            SchemaFingerprint::Fingerprint(fingerprint) => {
                serialize_tagged(serializer, "fingerprint", &Inspection(fingerprint))
            }
            SchemaFingerprint::NoSchema => serialize_tagged(serializer, "no_schema", &()),
        }
    }
}
