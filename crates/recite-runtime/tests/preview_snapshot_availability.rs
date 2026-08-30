use recite_compiler::{CompileInput, CompileOptions, compile_inputs_with_schema};
use recite_core::load_schema_manifest_str;
use recite_runtime::{
    ConditionAnswer, ConditionValue, PreviewEvent, PreviewInputs, PreviewOptions, PreviewSession,
};

#[test]
fn hidden_choice_and_reason_tree_survive_prompt_snapshot_restore() -> Result<(), String> {
    let asset = asset_with_reason()?;
    let mut source = PreviewSession::new(&asset, None, PreviewOptions::new())
        .map_err(|error| format!("source: {error:?}"))?;
    let first = source.step(PreviewInputs::new());
    let request = match first.events() {
        [PreviewEvent::ConditionRequested(request)] => request.clone(),
        events => return Err(format!("expected availability query, got {events:?}")),
    };
    let after_hidden = source.answer(
        request.id(),
        ConditionAnswer::Value(ConditionValue::Bool(false)),
        PreviewInputs::new(),
    );
    let request = match after_hidden.events() {
        [
            PreviewEvent::ConditionResult { .. },
            PreviewEvent::ConditionRequested(request),
        ] => request.clone(),
        events => return Err(format!("expected visible-choice query, got {events:?}")),
    };
    let output = source.answer(
        request.id(),
        ConditionAnswer::Value(ConditionValue::Bool(false)),
        PreviewInputs::new(),
    );
    if !matches!(
        output.events(),
        [
            PreviewEvent::ConditionResult { .. },
            PreviewEvent::Prompt(_)
        ]
    ) {
        return Err(format!(
            "expected condition result and prompt, got {:?}",
            output.events()
        ));
    }
    let Some(PreviewEvent::Prompt(prompt)) = output.events().last() else {
        return Err(format!("expected prompt event, got {:?}", output.events()));
    };
    assert_eq!(
        prompt
            .identity()
            .choices()
            .iter()
            .map(|choice| choice.as_str())
            .collect::<Vec<_>>(),
        ["12345678901234567891", "12345678901234567892"]
    );
    let locked = &prompt.choices()[0].availability;
    assert!(!locked.is_available);
    assert!(locked.primary_reason.is_some());
    assert!(locked.reason_tree.is_some());
    let snapshot = source
        .snapshot()
        .map_err(|error| format!("snapshot: {error:?}"))?;
    let mut restored = PreviewSession::new(&asset, None, PreviewOptions::new())
        .map_err(|error| format!("restored: {error:?}"))?;
    restored
        .restore(
            recite_runtime::PreviewSnapshot::decode(
                &snapshot
                    .encode()
                    .map_err(|error| format!("encode: {error:?}"))?,
            )
            .map_err(|error| format!("decode: {error:?}"))?,
        )
        .map_err(|error| format!("restore: {error:?}"))?;
    assert_eq!(restored.state(), source.state());
    assert!(
        restored
            .trace()
            .events()
            .iter()
            .any(|event| matches!(event, PreviewEvent::Restored))
    );
    assert!(
        restored
            .transcript()
            .events()
            .iter()
            .any(|event| matches!(event, recite_runtime::PreviewTranscriptEvent::Restored))
    );
    Ok(())
}

fn asset_with_reason() -> Result<recite_core::CompiledDialogue, String> {
    let schema = load_schema_manifest_str(
        "fixtures/schema/valid/generated_manifest.json",
        include_str!("../../../fixtures/schema/valid/generated_manifest.json"),
    )
    .schema
    .ok_or_else(|| "generated schema is missing".to_owned())?;
    let report = compile_inputs_with_schema(
        [CompileInput::new(
            "dialogue/preview.recite",
            concat!(
                ":: start default\n",
                ":if trust_gte(hazel, rhea, 3)\n",
                "  > hidden_line@12345678901234567893\n    Hidden.\n",
                "    ? hidden_choice@12345678901234567894\n      Hidden choice.\n      -> END\n",
                "> prompt@12345678901234567890\n  Choose.\n",
                "  ? locked@12345678901234567891 requires=(trust_gte(hazel, rhea, 3)) reason=innkeeper_trust_hint\n",
                "    Locked.\n    -> END\n",
                "  ? leave@12345678901234567892\n    Leave.\n    -> END\n",
            ),
        )],
        CompileOptions::new(
            recite_core::CompilerVersion::new("0.0.1")
                .map_err(|error| format!("version: {error:?}"))?,
            recite_core::CompiledAssetId::new("dialogue/preview.recitec")
                .map_err(|error| format!("asset id: {error:?}"))?,
            recite_core::SourceMapId::new("dialogue/preview.map")
                .map_err(|error| format!("map id: {error:?}"))?,
            schema.canonical_fingerprint(),
        ),
        &schema,
    )
    .map_err(|error| format!("compile: {error:?}"))?;
    if !report.diagnostics.is_empty() {
        return Err(format!("diagnostics: {:?}", report.diagnostics));
    }
    report
        .asset
        .ok_or_else(|| "compiled asset is missing".to_owned())
        .map(|asset| asset.dialogue)
}
