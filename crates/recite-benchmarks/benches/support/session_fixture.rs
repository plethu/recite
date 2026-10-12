//! One checked session fixture and condition policy for session benchmarks.
use recite_adapter::LoadedDialogue;
use recite_compiler::compile::{CompileInput, CompileOptions, compile_inputs};
use recite_core::{
    ChoiceId,
    compiled::{CompiledAssetId, CompilerVersion, SchemaFingerprint, SourceMapId},
};
use recite_runtime::{ConditionEvaluationError, ConditionQuery, ConditionValue};

type SessionResult<T> = Result<T, Box<dyn std::error::Error>>;

fn error(message: impl Into<String>) -> Box<dyn std::error::Error> {
    message.into().into()
}

pub(super) fn load(
    deferred_effects: bool,
    source_count: usize,
) -> SessionResult<(LoadedDialogue, ChoiceId)> {
    assert!(source_count > 0, "session workload needs a source file");
    let source = if deferred_effects {
        include_str!("../../../../fixtures/recite/valid/benchmarks/session_deferred.recite")
    } else {
        include_str!("../../../../fixtures/recite/valid/benchmarks/session_history.recite")
    };
    let mut inputs = vec![CompileInput::new("session.recite", source)];
    for index in 1..source_count {
        inputs.push(CompileInput::new(
            format!("shard_{index}.recite"),
            format!(":: shard_{index}\n-> END\n"),
        ));
    }
    let report = compile_inputs(
        inputs,
        CompileOptions::new(
            CompilerVersion::new("benchmarks")?,
            CompiledAssetId::new("session.recitec")?,
            SourceMapId::new("session.recitec.map")?,
            SchemaFingerprint::NoSchema,
        ),
    )?;
    if !report.diagnostics.is_empty() {
        return Err(error(format!(
            "session fixture diagnostics: {:?}",
            report.diagnostics
        )));
    }
    let compiled = report.asset.ok_or_else(|| error("missing session asset"))?;
    let asset = LoadedDialogue::from_bytes(&compiled.messagepack)?;
    assert_eq!(asset.dialogue().sources.len(), source_count);
    Ok((asset, ChoiceId::new("11111111111111111111")?))
}

pub(super) fn ready(query: ConditionQuery<'_>) -> Result<ConditionValue, ConditionEvaluationError> {
    if query.function() == "ready" {
        Ok(ConditionValue::Bool(true))
    } else {
        Err(ConditionEvaluationError::new(
            "unexpected benchmark condition",
        ))
    }
}
