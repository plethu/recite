use recite_compiler::{CompileInput, CompileOptions, compile_inputs};
use recite_core::{
    CompiledAssetId, CompilerVersion, SchemaFingerprint, SourceMapId,
    decode_compiled_dialogue_messagepack,
};
use recite_runtime::{
    DialogueEvent, DialogueSessionOptions, EmptyDialogueContext, LocaleError, LocaleProvider,
    LocaleResolution, PluralResolution, TextDomain, choose, next_with, restore_session,
    snapshot_session, start_scene_with_options,
};

struct LegacyLocaleProvider;

impl LocaleProvider for LegacyLocaleProvider {
    fn lookup(
        &self,
        id: &str,
        _source_text: &str,
        _domain: TextDomain,
        _locale: &recite_core::LocaleId,
        _variant: Option<&str>,
    ) -> Result<Option<String>, LocaleError> {
        Ok(match id {
            "8843fd6f53f020a12b31" => Some("Bonjour.".to_owned()),
            "b2c08cc280c726da34bf" => Some("Choisir.".to_owned()),
            _ => None,
        })
    }

    fn resolve_plural(
        &self,
        _id: &str,
        _source_singular: &str,
        _source_plural: &str,
        _count: i64,
        _domain: TextDomain,
        _locale: &recite_core::LocaleId,
        _variant: Option<&str>,
    ) -> Result<PluralResolution, LocaleError> {
        Ok(PluralResolution {
            template: None,
            selected_arm: None,
            matched_locale: None,
            matched_context: None,
            matched_key: None,
            attempts: Vec::new(),
        })
    }
}

#[test]
fn legacy_wire_rows_preserve_braced_line_and_choice_text_across_snapshot() {
    let source = concat!(
        ":: start default\n",
        "> intro@8843fd6f53f020a12b31\n",
        "  Hello.\n",
        "  ? ask@b2c08cc280c726da34bf\n",
        "    Choose.\n",
        "    -> END\n",
        "-> END\n",
    );
    let report = compile_inputs(
        [CompileInput::new("dialogue/legacy.recite", source)],
        CompileOptions::new(
            CompilerVersion::new("0.0.1")
                .unwrap_or_else(|error| panic!("valid compiler version: {error}")),
            CompiledAssetId::new("legacy")
                .unwrap_or_else(|error| panic!("valid asset id: {error}")),
            SourceMapId::new("legacy-map")
                .unwrap_or_else(|error| panic!("valid source map id: {error}")),
            SchemaFingerprint::NoSchema,
        ),
    )
    .unwrap_or_else(|error| panic!("legacy source compiles: {error:?}"));
    let mut wire: serde_value::Value = rmp_serde::from_slice(
        &report
            .asset
            .unwrap_or_else(|| panic!("legacy source produces an asset"))
            .messagepack,
    )
    .unwrap_or_else(|error| panic!("current wire decodes for test mutation: {error}"));
    let serde_value::Value::Seq(fields) = &mut wire else {
        panic!("compiled dialogue is a tuple");
    };
    let serde_value::Value::Seq(lines) = &mut fields[6] else {
        panic!("compiled lines are a sequence");
    };
    let serde_value::Value::Seq(line) = &mut lines[0] else {
        panic!("compiled line is a tuple");
    };
    line[1] = serde_value::Value::String("Hello {unbound}.".to_owned());
    line.truncate(5);
    let serde_value::Value::Seq(choices) = &mut fields[7] else {
        panic!("compiled choices are a sequence");
    };
    let serde_value::Value::Seq(choice) = &mut choices[0] else {
        panic!("compiled choice is a tuple");
    };
    choice[1] = serde_value::Value::String("Choose {unbound}.".to_owned());
    choice.truncate(9);
    let bytes =
        rmp_serde::to_vec(&wire).unwrap_or_else(|error| panic!("legacy wire encodes: {error}"));
    let asset = decode_compiled_dialogue_messagepack(&bytes)
        .unwrap_or_else(|error| panic!("legacy wire decodes: {error}"));

    let provider = LegacyLocaleProvider;
    let locale = recite_core::LocaleId::new("en-GB".to_owned())
        .unwrap_or_else(|error| panic!("legacy locale is valid: {error}"));
    let mut session = start_scene_with_options(
        &asset,
        None,
        DialogueSessionOptions::new().with_locale(locale),
    )
    .unwrap_or_else(|error| panic!("legacy scene starts: {error}"));
    let prompt_event = next_with(
        &asset,
        &mut session,
        &EmptyDialogueContext,
        LocaleResolution::new().with_provider(&provider),
    )
    .unwrap_or_else(|error| panic!("legacy prompt traverses: {error}"));
    let DialogueEvent::Prompt {
        line: Some(line),
        choices,
    } = prompt_event
    else {
        panic!("expected legacy prompt event");
    };
    assert_eq!(line.source_text, "Hello {unbound}.");
    assert_eq!(choices[0].source_text, "Choose {unbound}.");
    assert_eq!(line.text, "Bonjour.");
    assert_eq!(choices[0].text, "Choisir.");
    let snapshot = snapshot_session(&session);
    let mut restored = restore_session(&asset, snapshot)
        .unwrap_or_else(|error| panic!("legacy snapshot restores: {error}"));
    choose(
        &asset,
        &mut restored,
        choices[0].id.clone(),
        &EmptyDialogueContext,
    )
    .unwrap_or_else(|error| panic!("legacy choice selects: {error}"));
}
