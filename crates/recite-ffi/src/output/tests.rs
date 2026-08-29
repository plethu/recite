use std::error::Error as _;
use std::io::{self, Write};

use recite_compiler::{CompileInput, CompileOptions, compile_inputs};
use recite_core::{
    CompiledAssetId, CompilerVersion, LineId, ScalarValue, SchemaFingerprint, SourceMapId,
};
use recite_runtime::{
    DialogueEvent, DialogueLine, DialoguePlural, DialoguePluralResolution,
    DialoguePluralResolutionOutcome, EmptyDialogueContext, InterpolationValues, LocaleResolution,
    PluralResolutionAttempt, PluralResolutionOutcome, next_with, start_scene,
};

use super::{FfiOutputEncodeError, encode_batch, encode_batch_output, encode_batch_to_writer};
use crate::ReciteStatus;

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _bytes: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "writer failed"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn output_encoder_retains_messagepack_source_error() {
    let mut writer = FailingWriter;
    let error = encode_batch_to_writer(Vec::new(), &mut writer)
        .expect_err("the injected writer failure must reach the encoder");

    let source = error.source().expect("typed encoder source is retained");
    assert!(
        source.downcast_ref::<rmp_serde::encode::Error>().is_some(),
        "source should remain the rmp-serde encode error"
    );
    assert!(
        error
            .to_string()
            .contains("failed to encode FFI output batch")
    );

    let Err((status, message)) = encode_batch_output(Vec::new(), failing_encode) else {
        panic!("the production output seam must map encoder failures");
    };
    assert_eq!(status, ReciteStatus::DialogueFault);
    assert!(message.contains("failed to encode FFI output batch"));
}

fn failing_encode(events: Vec<DialogueEvent>) -> Result<Vec<u8>, FfiOutputEncodeError> {
    let mut writer = FailingWriter;
    encode_batch_to_writer(events, &mut writer).map(|()| Vec::new())
}

#[test]
fn empty_and_normal_batches_use_the_same_messagepack_encoder() {
    let empty = encode_batch(Vec::new()).expect("empty batch encodes");
    let normal = encode_batch(vec![DialogueEvent::End {
        deferred_effects: Vec::new(),
    }])
    .expect("normal batch encodes");

    let empty: serde_json::Value = rmp_serde::from_slice(&empty).expect("empty batch decodes");
    let normal: serde_json::Value = rmp_serde::from_slice(&normal).expect("normal batch decodes");
    assert_eq!(empty["batch_format_version"], 0);
    assert_eq!(normal["batch_format_version"], 0);
    assert_eq!(empty["events"].as_array().expect("events array").len(), 0);
    assert_eq!(normal["events"][0]["kind"], "end");
}

#[test]
fn plural_line_metadata_survives_ffi_conversion_without_translation_template() {
    let line = DialogueLine {
        id: LineId::new("22222222222222222222").expect("line ID"),
        source_text: "many {count}".to_owned(),
        text: "many 0".to_owned(),
        speaker: None,
        metadata: Vec::new(),
        plural: Some(DialoguePlural {
            singular_source_text: "one".to_owned(),
            plural_source_text: "many {count}".to_owned(),
            count: 0,
            selected_arm: 1,
            resolution: DialoguePluralResolution {
                attempts: vec![PluralResolutionAttempt {
                    locale: "fr-CA".to_owned(),
                    context: "22222222222222222222".to_owned(),
                    key: "22222222222222222222".to_owned(),
                    selected_arm: Some(0),
                    outcome: PluralResolutionOutcome::MissingEntry,
                }],
                matched_locale: None,
                matched_context: None,
                matched_key: None,
                matched_arm: None,
                source_fallback_arm: Some(1),
                outcome: DialoguePluralResolutionOutcome::EnglishSourceFallback,
            },
        }),
    };
    let encoded = encode_batch(vec![DialogueEvent::Line(line)]).expect("batch encodes");
    let value: serde_json::Value = rmp_serde::from_slice(&encoded).expect("batch decodes");
    assert_eq!(value["events"][0]["plural"]["count"], 0);
    assert_eq!(
        value["events"][0]["plural"]["resolution"]["outcome"],
        "english_source_fallback"
    );
    assert_eq!(
        value["events"][0]["plural"]["resolution"]["attempts"][0]["outcome"],
        "missing_entry"
    );
    assert!(value["events"][0]["plural"].get("template").is_none());
}

#[test]
fn escaped_plural_braces_survive_runtime_and_ffi_serialization() {
    let source = concat!(
        ":: start default\n",
        "> letters@22222222222222222222 bind=(count:int=$remaining)\n",
        "  You have \\{count\\} letter.\n",
        "  | You have {count} letters.\n",
        "-> END\n",
    );
    let asset = compile_inputs(
        [CompileInput::new("dialogue/escaped.recite", source)],
        CompileOptions::new(
            CompilerVersion::new("0.0.1").expect("compiler version"),
            CompiledAssetId::new("escaped").expect("asset ID"),
            SourceMapId::new("escaped-map").expect("source map ID"),
            SchemaFingerprint::NoSchema,
        ),
    )
    .expect("escaped plural source compiles")
    .asset
    .expect("escaped plural source emits an asset")
    .dialogue;
    let mut session = start_scene(&asset, None).expect("session starts");
    let mut values = InterpolationValues::new();
    values.insert("remaining".to_owned(), ScalarValue::from(1_i64));
    let event = next_with(
        &asset,
        &mut session,
        &EmptyDialogueContext,
        LocaleResolution::new().with_values(&values),
    )
    .expect("plural event resolves");
    let DialogueEvent::Line(line) = &event else {
        panic!("expected plural line");
    };
    assert_eq!(line.source_text, "You have {count} letter.");
    assert_eq!(line.text, "You have {count} letter.");
    assert_eq!(
        line.plural
            .as_ref()
            .expect("plural metadata")
            .singular_source_text,
        r"You have \{count\} letter."
    );

    let encoded = encode_batch(vec![event]).expect("FFI batch encodes");
    let value: serde_json::Value = rmp_serde::from_slice(&encoded).expect("batch decodes");
    assert_eq!(
        value["events"][0]["source_text"],
        "You have {count} letter."
    );
    assert_eq!(
        value["events"][0]["plural"]["singular_source_text"],
        r"You have \{count\} letter."
    );
}
