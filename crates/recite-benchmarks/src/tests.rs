use recite_core::{
    AvailabilityReasonId, ChoiceId, CompiledAssetId, EffectId, LineId, MetadataEntry, ScalarValue,
    SourcePosition, SourceSpan, Value,
};
use recite_runtime::{
    ChoiceAvailability, ChoiceAvailabilityReason, ChoiceAvailabilityReasonOrigin,
    ChoiceAvailabilityReasonTree, ChoiceAvailabilityReasonValue, ChoiceEchoMode, DialogueChoice,
    DialogueEffectMode, DialogueEffectRequest, DialogueError, DialogueLine, DialoguePlural,
    DialoguePluralResolution, EffectAck, PreviewConditionRequestId, PreviewError, PreviewEvent,
    PreviewOutput,
};

fn digest(event: &PreviewEvent) -> String {
    let mut hasher = blake3::Hasher::new();
    crate::preview_hash::hash_event(event, &mut hasher);
    hasher.finalize().to_hex().to_string()
}

fn state_digest(output: &PreviewOutput) -> String {
    let mut hasher = blake3::Hasher::new();
    crate::preview_hash::hash_output_state(output, &mut hasher);
    hasher.finalize().to_hex().to_string()
}

fn span(column: u32) -> SourceSpan {
    SourceSpan::point(
        "fixture.recite",
        SourcePosition::new(2, column).expect("test span positions are valid"),
    )
}

#[test]
fn metadata_values_are_part_of_line_evidence() {
    let mut line = DialogueLine {
        id: LineId::new("line").expect("test ID is valid"),
        source_text: "source".to_owned(),
        text: "text".to_owned(),
        speaker: None,
        metadata: vec![MetadataEntry::new(
            "tone",
            Value::Scalar(ScalarValue::String("quiet".to_owned())),
        )],
        plural: None,
    };
    let first = digest(&PreviewEvent::Line(line.clone()));
    line.metadata[0].value = Value::Scalar(ScalarValue::String("loud".to_owned()));
    assert_ne!(first, digest(&PreviewEvent::Line(line)));
}

#[test]
fn effect_source_spans_are_part_of_effect_evidence() {
    let mut effect = DialogueEffectRequest {
        id: EffectId::new("set_flag").expect("test ID is valid"),
        mode: DialogueEffectMode::Immediate,
        function: "set_flag".to_owned(),
        args: Vec::new(),
        source_span: span(1),
    };
    let first = digest(&PreviewEvent::EffectRequested(effect.clone()));
    effect.source_span = span(4);
    assert_ne!(first, digest(&PreviewEvent::EffectRequested(effect)));
}

#[test]
fn plural_provenance_is_part_of_line_evidence() {
    let mut line = DialogueLine {
        id: LineId::new("line").expect("test ID is valid"),
        source_text: "one".to_owned(),
        text: "one".to_owned(),
        speaker: None,
        metadata: Vec::new(),
        plural: Some(DialoguePlural {
            singular_source_text: "one".to_owned(),
            plural_source_text: "many".to_owned(),
            count: 2,
            selected_arm: 1,
            resolution: DialoguePluralResolution {
                attempts: Vec::new(),
                matched_locale: Some("en".to_owned()),
                matched_context: Some("dialogue".to_owned()),
                matched_key: Some("line".to_owned()),
                matched_arm: Some(1),
                source_fallback_arm: None,
                outcome: recite_runtime::DialoguePluralResolutionOutcome::Translated,
            },
        }),
    };
    let first = digest(&PreviewEvent::Line(line.clone()));
    line.plural
        .as_mut()
        .expect("test plural exists")
        .resolution
        .matched_key = Some("changed".to_owned());
    assert_ne!(first, digest(&PreviewEvent::Line(line)));
}

#[test]
fn error_payloads_are_part_of_error_evidence() {
    let first = PreviewEvent::Error(PreviewError::ConditionFailed {
        request_id: PreviewConditionRequestId::new(1),
        reason: "first".to_owned(),
    });
    let second = PreviewEvent::Error(PreviewError::ConditionFailed {
        request_id: PreviewConditionRequestId::new(1),
        reason: "second".to_owned(),
    });
    assert_ne!(digest(&first), digest(&second));
}

#[test]
fn acknowledgement_payloads_are_part_of_ack_evidence() {
    let effect_id = EffectId::new("set_flag").expect("test ID is valid");
    let first = PreviewEvent::EffectAcknowledged {
        effect_id: effect_id.clone(),
        ack: EffectAck::Completed,
    };
    let second = PreviewEvent::EffectAcknowledged {
        effect_id,
        ack: EffectAck::Failed {
            reason: "failed".to_owned(),
        },
    };
    assert_ne!(digest(&first), digest(&second));
}

#[test]
fn pending_effect_error_variants_have_distinct_evidence_tags() {
    let effect_id = EffectId::new("set_flag").expect("test ID is valid");
    let pending = PreviewEvent::Error(PreviewError::Runtime(DialogueError::EffectPending {
        effect: effect_id.clone(),
    }));
    let absent = PreviewEvent::Error(PreviewError::Runtime(DialogueError::NoEffectPending {
        effect: effect_id,
    }));
    assert_ne!(digest(&pending), digest(&absent));
}

fn choice_with_reason_tree_and_echo() -> DialogueChoice {
    DialogueChoice {
        id: ChoiceId::new("choice").expect("test ID is valid"),
        source_text: "choose".to_owned(),
        text: "Choose".to_owned(),
        metadata: Vec::new(),
        availability: ChoiceAvailability {
            is_available: false,
            primary_reason: Some(ChoiceAvailabilityReason {
                id: AvailabilityReasonId::new("locked").expect("test ID is valid"),
                source_text: "locked".to_owned(),
                text: "Not ready".to_owned(),
                origin: Some(ChoiceAvailabilityReasonOrigin::ConditionCall {
                    function: "has_key".to_owned(),
                    args: vec![ChoiceAvailabilityReasonValue::Boolean(false)],
                }),
                args: Vec::new(),
            }),
            reason_tree: Some(ChoiceAvailabilityReasonTree::RequirementSourceText(
                "has_key".to_owned(),
            )),
        },
        echo: ChoiceEchoMode::None,
    }
}

fn choice_digest(choice: &DialogueChoice) -> blake3::Hash {
    let mut first_hasher = blake3::Hasher::new();
    crate::preview_hash_dialogue::hash_choice(&mut first_hasher, choice);
    first_hasher.finalize()
}

#[test]
fn choice_reason_tree_changes_choice_evidence_independently() {
    let mut choice = choice_with_reason_tree_and_echo();
    let first = choice_digest(&choice);
    choice.availability.reason_tree = Some(ChoiceAvailabilityReasonTree::RequirementSourceText(
        "changed".to_owned(),
    ));
    assert_ne!(first, choice_digest(&choice));
}

#[test]
fn choice_echo_changes_choice_evidence_independently() {
    let mut choice = choice_with_reason_tree_and_echo();
    let first = choice_digest(&choice);
    choice.echo = ChoiceEchoMode::SelectedText;
    assert_ne!(first, choice_digest(&choice));
}

#[test]
fn preview_output_state_digest_changes_for_projected_fields()
-> Result<(), Box<dyn std::error::Error>> {
    let project = crate::preview::PreviewProject::load(crate::BenchmarkFixture::Synthetic(
        crate::BenchmarkScale::Tiny,
    ))?;

    let mut default_preview = project.start()?;
    let default_output = default_preview.step(project.inputs());
    let default_digest = state_digest(&default_output);

    let mut source_only_preview = recite_runtime::PreviewSession::new(
        &project.asset,
        None,
        recite_runtime::PreviewOptions::new(),
    )?;
    let source_only_output = source_only_preview.step(recite_runtime::PreviewInputs::new());
    assert_ne!(default_digest, state_digest(&source_only_output));

    let mut explicit_block_preview = recite_runtime::PreviewSession::new(
        &project.asset,
        Some(project.runtime_fixture.first_prompt_block().as_str()),
        recite_runtime::PreviewOptions::new().with_locale(project.runtime_fixture.locale()),
    )?;
    let explicit_block_output = explicit_block_preview.step(project.inputs());
    assert_ne!(default_digest, state_digest(&explicit_block_output));

    let mut candidate = project.asset.clone();
    candidate.header.asset_id = CompiledAssetId::new("replacement-asset")?;
    let restart_output = default_preview.assess_asset(&candidate)?;
    assert_ne!(default_digest, state_digest(&restart_output));

    let mut prompt_preview = project.at_first_prompt()?;
    let prompt_state_output = prompt_preview.step(project.inputs());
    assert_ne!(default_digest, state_digest(&prompt_state_output));

    let mut selected_preview = project.at_first_prompt()?;
    let choice = match selected_preview.state().status() {
        recite_runtime::PreviewStatus::WaitingForChoice { prompt } => prompt
            .choices()
            .first()
            .expect("tiny preview prompt has a choice")
            .id
            .clone(),
        _ => return Err("tiny preview did not reach a choice".into()),
    };
    let selected_output = selected_preview.choose(choice, project.inputs());
    assert_ne!(
        state_digest(&prompt_state_output),
        state_digest(&selected_output)
    );

    let mut deferred_preview = project.after_first_choice()?;
    let deferred_output = deferred_preview.step(project.inputs());
    assert!(!deferred_output.state().deferred_effects().is_empty());
    assert_ne!(default_digest, state_digest(&deferred_output));

    Ok(())
}

#[test]
fn preview_output_state_digest_is_hashed_at_each_boundary() -> Result<(), Box<dyn std::error::Error>>
{
    let project = crate::preview::PreviewProject::load(crate::BenchmarkFixture::Synthetic(
        crate::BenchmarkScale::Tiny,
    ))?;
    let mut preview = project.start()?;
    let first = preview.step(project.inputs());
    let second = preview.step(project.inputs());
    let mut hasher = blake3::Hasher::new();
    crate::preview_hash::hash_output_state(&first, &mut hasher);
    crate::preview_hash::hash_output_state(&second, &mut hasher);
    let two_boundaries = hasher.finalize();

    let mut one_boundary_hasher = blake3::Hasher::new();
    crate::preview_hash::hash_output_state(&first, &mut one_boundary_hasher);
    assert_ne!(two_boundaries, one_boundary_hasher.finalize());
    Ok(())
}
