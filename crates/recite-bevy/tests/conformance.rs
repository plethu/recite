use std::fs;
use std::io;
use std::path::PathBuf;

use bevy_app::App;
use bevy_asset::{AssetPlugin, Assets};
use bevy_ecs::message::{MessageCursor, Messages};
use recite_bevy::{
    AdapterErrorKind, ReciteCatalog, ReciteConditions, ReciteDialogueAsset, ReciteInterpolation,
    ReciteOutput, ReciteOutputValue, RecitePlugin, ReciteRequest,
};
use recite_compiler::compile::{CompileInput, CompileOptions, compile_inputs};
use recite_core::{
    ChoiceId, EffectId, LocaleId, ScalarValue,
    compiled::{
        CompiledAssetId, CompilerVersion, ContentFingerprint, SchemaFingerprint, SourceMapId,
    },
};
use recite_runtime::{
    ConditionEvaluationError, ConditionValue, DialogueEvent, DialoguePluralResolutionOutcome,
    EffectAck, localisation::PluralResolutionOutcome,
};
use serde_json::Value;

#[path = "conformance/support.rs"]
mod support;
use support::*;

#[test]
fn plural_line_structured_metadata_adapter_runner_required() -> TestResult<()> {
    let case = scenario("plural_line_structured_metadata_adapter_runner_required")?;
    assert_eq!(case["execution_mode"], "adapter_runner_required");
    let fixture = case["steps"][0]["operation"]["fixture"]
        .as_str()
        .expect("fixture");
    let expected = &case["steps"][2]["expect"]["line_plural"];
    let mut app = App::new();
    app.add_plugins((AssetPlugin::default(), RecitePlugin));
    app.world_mut()
        .resource_mut::<ReciteInterpolation>()
        .0
        .insert("count".to_owned(), ScalarValue::Integer(2));
    app.world_mut()
        .resource_mut::<ReciteCatalog>()
        .0
        .set_plural_forms("fr-FR", "nplurals=2; plural=(n != 1);")
        .expect("plural rule");
    app.world_mut()
        .resource_mut::<ReciteCatalog>()
        .0
        .insert_plural(
            "fr-FR",
            "5fcf9a1f7b20211f4a92",
            "You have one letter.",
            "You have {count} letters.",
            vec![
                "Vous avez une lettre.".to_owned(),
                "Vous avez {count} lettres.".to_owned(),
            ],
            None,
        )
        .expect("plural entry");
    let handle = app
        .world_mut()
        .resource_mut::<Assets<ReciteDialogueAsset>>()
        .add(fixture_asset(fixture)?);
    app.update();
    app.world_mut()
        .resource_mut::<Messages<ReciteRequest>>()
        .write(ReciteRequest::Start {
            asset: handle,
            block_id: None,
            locale: Some(LocaleId::new("fr-FR").expect("locale")),
            variant: None,
        });
    app.update();
    let mut cursor = MessageCursor::<ReciteOutput>::default();
    let output = cursor
        .read(app.world().resource::<Messages<ReciteOutput>>())
        .last()
        .expect("output");
    let ReciteOutputValue::Dialogue(events) = &output.value else {
        panic!("dialogue batch");
    };
    let Some(DialogueEvent::Line(line)) = events.first() else {
        panic!("plural line");
    };
    let plural = line.plural.as_ref().expect("structured plural metadata");
    assert_eq!(
        plural.singular_source_text,
        expected["singular_source_text"]
    );
    assert_eq!(plural.plural_source_text, expected["plural_source_text"]);
    assert_eq!(plural.count, expected["count"].as_i64().expect("count"));
    assert_eq!(
        plural.selected_arm,
        expected["selected_arm"].as_u64().expect("arm") as usize
    );
    assert_eq!(
        plural.resolution.matched_locale.as_deref(),
        expected["resolution"]["matched_locale"].as_str()
    );
    assert_eq!(
        plural.resolution.matched_context.as_deref(),
        expected["resolution"]["matched_context"].as_str()
    );
    assert_eq!(
        plural.resolution.matched_key.as_deref(),
        expected["resolution"]["matched_key"].as_str()
    );
    assert_eq!(
        plural.resolution.matched_arm,
        expected["resolution"]["matched_arm"]
            .as_u64()
            .map(|arm| arm as usize)
    );
    assert_eq!(
        plural.resolution.outcome,
        DialoguePluralResolutionOutcome::Translated
    );
    assert_eq!(
        plural.resolution.attempts.len(),
        expected["resolution"]["attempts"]
            .as_array()
            .expect("attempts")
            .len()
    );
    for (actual, expected) in plural.resolution.attempts.iter().zip(
        expected["resolution"]["attempts"]
            .as_array()
            .expect("attempts"),
    ) {
        assert_eq!(actual.locale, expected["locale"]);
        assert_eq!(actual.context, expected["context"]);
        assert_eq!(actual.key, expected["key"]);
        assert_eq!(
            actual.selected_arm,
            expected["selected_arm"].as_u64().map(|arm| arm as usize)
        );
        assert_eq!(
            actual.outcome,
            match expected["outcome"].as_str().expect("outcome") {
                "matched" => PluralResolutionOutcome::Matched,
                "missing_entry" => PluralResolutionOutcome::MissingEntry,
                "missing_translation" => PluralResolutionOutcome::MissingTranslation,
                "missing_plural_forms" => PluralResolutionOutcome::MissingPluralForms,
                other => panic!("unknown expected outcome {other}"),
            }
        );
    }
    assert_eq!(line.text, "Vous avez 2 lettres.");
    Ok(())
}

#[test]
fn localisation_error_adapter_runner_required() -> TestResult<()> {
    let case = scenario("localisation_error_adapter_runner_required")?;
    assert_eq!(case["execution_mode"], "adapter_runner_required");
    let fixture = case["steps"][0]["operation"]["fixture"]
        .as_str()
        .expect("fixture");
    let _asset = fixture_asset(fixture)?;
    let mut app = App::new();
    app.add_plugins((AssetPlugin::default(), RecitePlugin));
    let error = app
        .world_mut()
        .resource_mut::<ReciteCatalog>()
        .0
        .set_plural_forms("fr-FR", "nplurals=2; plural=(n == 42 ? 2 : 0);")
        .expect_err("malformed catalog must fail at import");
    assert_eq!(error.kind(), AdapterErrorKind::Localisation);
    assert_eq!(error.code(), case["steps"][1]["expect"]["error_category"]);
    Ok(())
}

#[test]
fn lifecycle_and_choice_error_cases_use_published_categories() -> TestResult<()> {
    let (mut app, handle) = app_with_surface()?;
    register_trusts(&mut app, false);
    let mut cursor = MessageCursor::<ReciteOutput>::default();
    let opening = send(&mut app, &mut cursor, start(handle.clone(), None))?;
    assert!(matches!(opening.value, ReciteOutputValue::Dialogue(_)));
    assert_case_error(
        send(&mut app, &mut cursor, start(handle.clone(), None))?,
        "session_already_active_error_second_start",
    )?;
    assert_case_error(
        send(
            &mut app,
            &mut cursor,
            ReciteRequest::SelectChoice(
                ChoiceId::new("missing_choice").expect("syntactically valid missing ID"),
            ),
        )?,
        "invalid_choice_error_unknown_choice_id",
    )?;
    assert_case_error(
        send(
            &mut app,
            &mut cursor,
            ReciteRequest::SelectChoice(
                ChoiceId::new("8706986735003ec9aaec").expect("unavailable choice"),
            ),
        )?,
        "unavailable_choice_error_conditioned_choice",
    )?;
    send(&mut app, &mut cursor, ReciteRequest::End)?;
    assert_case_error(
        send(
            &mut app,
            &mut cursor,
            ReciteRequest::SelectChoice(ChoiceId::new("3f481d9991fa22e23b0c").expect("choice")),
        )?,
        "no_active_session_error_after_explicit_end",
    )?;
    assert_case_error(
        send(&mut app, &mut cursor, start(handle, Some("missing_block")))?,
        "unknown_start_block_error_missing_block",
    )?;
    Ok(())
}

#[test]
fn condition_failures_are_transactional_and_typed() -> TestResult<()> {
    for (id, handler) in [
        ("missing_condition_handler_error_no_registered_handler", 0),
        ("condition_evaluation_error_handler_failure", 1),
        ("invalid_condition_result_error_wrong_result_type", 2),
    ] {
        let (mut app, handle) = app_with_surface()?;
        if handler == 1 {
            app.world_mut()
                .resource_mut::<ReciteConditions>()
                .register("trusts", |_, _| {
                    Err(ConditionEvaluationError::new(
                        "condition service unavailable",
                    ))
                });
        } else if handler == 2 {
            app.world_mut()
                .resource_mut::<ReciteConditions>()
                .register("trusts", |_, _| {
                    Ok(ConditionValue::EnumVariant("ready".to_owned()))
                });
        }
        let mut cursor = MessageCursor::<ReciteOutput>::default();
        let output = send(&mut app, &mut cursor, start(handle, None))?;
        assert_case_error(output, id)?;
        assert!(
            !app.world()
                .resource::<recite_bevy::ReciteOwner>()
                .has_active_session(),
            "{id}: failed initial batch must not publish an active session"
        );
    }
    Ok(())
}

#[test]
fn blocking_ack_and_stale_choice_use_published_categories() -> TestResult<()> {
    let (mut app, handle) = app_with_surface()?;
    register_trusts(&mut app, true);
    let mut cursor = MessageCursor::<ReciteOutput>::default();
    send(&mut app, &mut cursor, start(handle, None))?;
    let blocking = send(
        &mut app,
        &mut cursor,
        ReciteRequest::SelectChoice(ChoiceId::new("3f481d9991fa22e23b0c").expect("work choice")),
    )?;
    let ReciteOutputValue::Dialogue(events) = blocking.value else {
        panic!("blocking batch");
    };
    let effect = events
        .iter()
        .find_map(|event| match event {
            DialogueEvent::Effect(effect) => Some(effect.id.clone()),
            _ => None,
        })
        .expect("blocking effect");
    assert_case_error(
        send(
            &mut app,
            &mut cursor,
            ReciteRequest::AcknowledgeEffect {
                effect: EffectId::new("effect:wrong").expect("syntactically valid effect ID"),
                ack: EffectAck::Completed,
            },
        )?,
        "effect_acknowledgement_error_wrong_pending_effect_id",
    )?;
    send(
        &mut app,
        &mut cursor,
        ReciteRequest::AcknowledgeEffect {
            effect,
            ack: EffectAck::Completed,
        },
    )?;
    assert_case_error(
        send(
            &mut app,
            &mut cursor,
            ReciteRequest::SelectChoice(
                ChoiceId::new("3f481d9991fa22e23b0c").expect("stale choice"),
            ),
        )?,
        "stale_choice_error_after_prompt_transition",
    )?;
    Ok(())
}

#[test]
fn compiled_decode_snapshot_format_and_schema_mismatch_cases() -> TestResult<()> {
    let expected = scenario("asset_load_or_decode_error_truncated_messagepack")?;
    let decode = ReciteDialogueAsset::from_bytes(&[0x93, 0x01]);
    assert_eq!(
        decode.expect_err("truncated bytes rejected").code(),
        expected["steps"][0]["expect"]["error_category"]
    );

    let (mut app, handle) = app_with_surface()?;
    register_trusts(&mut app, false);
    let mut cursor = MessageCursor::<ReciteOutput>::default();
    send(&mut app, &mut cursor, start(handle.clone(), None))?;
    let saved = send(&mut app, &mut cursor, ReciteRequest::Snapshot)?;
    let ReciteOutputValue::Snapshot(snapshot) = saved.value else {
        panic!("snapshot");
    };
    send(&mut app, &mut cursor, ReciteRequest::End)?;
    assert_case_error(
        send(
            &mut app,
            &mut cursor,
            ReciteRequest::Restore {
                asset: handle,
                snapshot: vec![0x93, 0x01],
                variant: None,
            },
        )?,
        "save_load_incompatibility_error_snapshot_format_mismatch",
    )?;

    let mut app = App::new();
    app.add_plugins((AssetPlugin::default(), RecitePlugin));
    register_trusts(&mut app, false);
    let first_schema =
        SchemaFingerprint::Fingerprint(ContentFingerprint::blake3(vec![1; 32]).expect("digest"));
    let second_schema =
        SchemaFingerprint::Fingerprint(ContentFingerprint::blake3(vec![2; 32]).expect("digest"));
    let first = app
        .world_mut()
        .resource_mut::<Assets<ReciteDialogueAsset>>()
        .add(fixture_asset_with_schema(RUNTIME_SURFACE, first_schema)?);
    let second = app
        .world_mut()
        .resource_mut::<Assets<ReciteDialogueAsset>>()
        .add(fixture_asset_with_schema(RUNTIME_SURFACE, second_schema)?);
    app.update();
    let mut cursor = MessageCursor::<ReciteOutput>::default();
    send(&mut app, &mut cursor, start(first, None))?;
    let saved = send(&mut app, &mut cursor, ReciteRequest::Snapshot)?;
    let ReciteOutputValue::Snapshot(schema_snapshot) = saved.value else {
        panic!("snapshot");
    };
    send(&mut app, &mut cursor, ReciteRequest::End)?;
    assert_case_error(
        send(
            &mut app,
            &mut cursor,
            ReciteRequest::Restore {
                asset: second,
                snapshot: schema_snapshot,
                variant: None,
            },
        )?,
        "schema_mismatch_error_on_restore",
    )?;
    assert!(!snapshot.is_empty());
    Ok(())
}

#[test]
fn plural_source_fallback_matches_reference_fixture_text() -> TestResult<()> {
    let case = scenario("plural_line_reference_driver_source_fallback")?;
    let fixture = case["steps"][0]["operation"]["fixture"]
        .as_str()
        .expect("fixture");
    let mut app = App::new();
    app.add_plugins((AssetPlugin::default(), RecitePlugin));
    app.world_mut()
        .resource_mut::<ReciteInterpolation>()
        .0
        .insert("count".to_owned(), ScalarValue::Integer(2));
    let handle = app
        .world_mut()
        .resource_mut::<Assets<ReciteDialogueAsset>>()
        .add(fixture_asset(fixture)?);
    app.update();
    let mut cursor = MessageCursor::<ReciteOutput>::default();
    let output = send(&mut app, &mut cursor, start(handle, None))?;
    let ReciteOutputValue::Dialogue(events) = output.value else {
        panic!("dialogue");
    };
    assert!(matches!(&events[0], DialogueEvent::Line(line) if line.text
        == case["steps"][3]["expect"]["line_text"].as_str().expect("expected line")));
    Ok(())
}
