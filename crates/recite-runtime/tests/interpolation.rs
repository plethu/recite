use std::cell::Cell;

use recite_compiler::{CompileInput, CompileOptions, compile_inputs};
use recite_core::{
    CompiledAssetId, CompilerVersion, ScalarValue, SchemaFingerprint, SourceMapId,
    decode_compiled_dialogue_messagepack,
};
use recite_runtime::{
    DialogueEvent, DialogueSessionOptions, DialogueTrace, EmptyDialogueContext,
    InterpolationValueProvider, InterpolationValues, LocaleError, LocaleProvider, LocaleResolution,
    PluralResolution, PluralResolutionAttempt, PluralResolutionOutcome, TextDomain, choose,
    next_with, start_scene, start_scene_with_options,
};

fn asset() -> recite_core::CompiledDialogue {
    let source = concat!(
        ":: start default\n",
        "> hello_001@8843fd6f53f020a12b31 bind=(name:string=$display)\n",
        "  Hello, {name}; {name}!\n",
        "-> END\n",
    );
    compile_inputs(
        [CompileInput::new("dialogue/interpolation.recite", source)],
        CompileOptions::new(
            CompilerVersion::new("0.0.1")
                .unwrap_or_else(|error| panic!("valid compiler version: {error}")),
            CompiledAssetId::new("interpolation")
                .unwrap_or_else(|error| panic!("valid asset id: {error}")),
            SourceMapId::new("interpolation-map")
                .unwrap_or_else(|error| panic!("valid source map id: {error}")),
            SchemaFingerprint::NoSchema,
        ),
    )
    .unwrap_or_else(|error| panic!("interpolation source compiles: {error:?}"))
    .asset
    .unwrap_or_else(|| panic!("interpolation source produces an asset"))
    .dialogue
}

#[test]
fn values_are_supplied_outside_session_and_rendered_after_lookup() {
    let asset = asset();
    let mut session = start_scene(&asset, None).unwrap();
    let mut values = InterpolationValues::new();
    values.insert("display".to_owned(), ScalarValue::from("Ada"));
    let event = next_with(
        &asset,
        &mut session,
        &EmptyDialogueContext,
        LocaleResolution::new().with_values(&values),
    )
    .unwrap();
    let DialogueEvent::Line(line) = event else {
        panic!("expected line");
    };
    assert_eq!(line.source_text, "Hello, {name}; {name}!");
    assert_eq!(line.text, "Hello, Ada; Ada!");
}

struct CountingValues {
    count: Cell<usize>,
}

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

fn plural_asset() -> recite_core::CompiledDialogue {
    let source = concat!(
        ":: start default\n",
        "> letters_001@8843fd6f53f020a12b31 bind=(count:int=$remaining)\n",
        "  You have one letter.\n",
        "  | You have {count} letters.\n",
        "-> END\n",
    );
    compile_inputs(
        [CompileInput::new("dialogue/plural.recite", source)],
        CompileOptions::new(
            CompilerVersion::new("0.0.1")
                .unwrap_or_else(|error| panic!("valid compiler version: {error}")),
            CompiledAssetId::new("plural")
                .unwrap_or_else(|error| panic!("valid asset id: {error}")),
            SourceMapId::new("plural-map")
                .unwrap_or_else(|error| panic!("valid source map id: {error}")),
            SchemaFingerprint::NoSchema,
        ),
    )
    .unwrap_or_else(|error| panic!("compile plural source: {error:?}"))
    .asset
    .unwrap_or_else(|| panic!("plural source emits an asset"))
    .dialogue
}

struct PluralProvider;

impl LocaleProvider for PluralProvider {
    fn lookup(
        &self,
        _id: &str,
        _source_text: &str,
        _domain: TextDomain,
        _locale: &recite_core::LocaleId,
        _variant: Option<&str>,
    ) -> Result<Option<String>, LocaleError> {
        Ok(None)
    }

    fn resolve_plural(
        &self,
        _id: &str,
        _source_singular: &str,
        _source_plural: &str,
        count: i64,
        _domain: TextDomain,
        _locale: &recite_core::LocaleId,
        _variant: Option<&str>,
    ) -> Result<PluralResolution, LocaleError> {
        Ok(PluralResolution {
            template: Some(if count == 1 {
                "Vous avez une lettre.".to_owned()
            } else {
                "Vous avez plusieurs lettres.".to_owned()
            }),
            selected_arm: Some(usize::from(count != 1)),
            matched_locale: Some("fr-FR".to_owned()),
            matched_context: Some("8843fd6f53f020a12b31".to_owned()),
            matched_key: Some("8843fd6f53f020a12b31".to_owned()),
            attempts: Vec::new(),
        })
    }
}

#[test]
fn plural_source_fallback_selects_english_forms_and_retains_selected_source() {
    let asset = plural_asset();
    let mut session = start_scene(&asset, None).unwrap();
    let mut values = InterpolationValues::new();
    values.insert("remaining".to_owned(), ScalarValue::from(2_i64));
    let DialogueEvent::Line(line) = next_with(
        &asset,
        &mut session,
        &recite_runtime::EmptyDialogueContext,
        LocaleResolution::new().with_values(&values),
    )
    .unwrap() else {
        panic!("expected plural line");
    };
    assert_eq!(line.source_text, "You have {count} letters.");
    assert_eq!(line.text, "You have 2 letters.");
    let plural = line.plural.expect("plural metadata");
    assert_eq!(plural.selected_arm, 1);
    assert_eq!(plural.resolution.source_fallback_arm, Some(1));
    assert_eq!(
        plural.resolution.outcome,
        recite_runtime::DialoguePluralResolutionOutcome::EnglishSourceFallback
    );
}

struct LocaleRuleWithoutTranslation;

impl LocaleProvider for LocaleRuleWithoutTranslation {
    fn lookup(
        &self,
        _id: &str,
        _source_text: &str,
        _domain: TextDomain,
        _locale: &recite_core::LocaleId,
        _variant: Option<&str>,
    ) -> Result<Option<String>, LocaleError> {
        Ok(None)
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
        // This represents a locale rule `(n > 1)`, which selects arm 0 for
        // count 0. It must not control English source fallback without a
        // matching translated entry.
        Ok(PluralResolution {
            template: None,
            selected_arm: None,
            matched_locale: None,
            matched_context: None,
            attempts: vec![PluralResolutionAttempt {
                locale: "fr".to_owned(),
                context: "8843fd6f53f020a12b31".to_owned(),
                key: "8843fd6f53f020a12b31".to_owned(),
                selected_arm: Some(0),
                outcome: PluralResolutionOutcome::MissingEntry,
            }],
            matched_key: None,
        })
    }
}

#[test]
fn source_fallback_ignores_unmatched_locale_plural_arm() {
    let asset = plural_asset();
    let mut session = start_scene_with_options(
        &asset,
        None,
        DialogueSessionOptions::new().with_locale(recite_core::LocaleId::new("fr").unwrap()),
    )
    .unwrap();
    let mut values = InterpolationValues::new();
    values.insert("remaining".to_owned(), ScalarValue::from(0_i64));
    let DialogueEvent::Line(line) = next_with(
        &asset,
        &mut session,
        &recite_runtime::EmptyDialogueContext,
        LocaleResolution::new()
            .with_provider(&LocaleRuleWithoutTranslation)
            .with_values(&values),
    )
    .unwrap() else {
        panic!("expected plural line");
    };
    assert_eq!(line.source_text, "You have {count} letters.");
    assert_eq!(line.text, "You have 0 letters.");
    assert_eq!(line.plural.expect("plural metadata").selected_arm, 1);
}

struct CountingPluralProvider {
    calls: Cell<usize>,
}

impl LocaleProvider for CountingPluralProvider {
    fn lookup(
        &self,
        _id: &str,
        _source_text: &str,
        _domain: TextDomain,
        _locale: &recite_core::LocaleId,
        _variant: Option<&str>,
    ) -> Result<Option<String>, LocaleError> {
        Ok(None)
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
        self.calls.set(self.calls.get() + 1);
        Ok(PluralResolution {
            template: Some("Vous avez une lettre.".to_owned()),
            selected_arm: Some(0),
            matched_locale: Some("fr".to_owned()),
            matched_context: Some("8843fd6f53f020a12b31".to_owned()),
            matched_key: Some("8843fd6f53f020a12b31".to_owned()),
            attempts: Vec::new(),
        })
    }
}

#[test]
fn plural_translation_uses_one_structured_provider_resolution_call() {
    let asset = plural_asset();
    let mut session = start_scene_with_options(
        &asset,
        None,
        DialogueSessionOptions::new().with_locale(recite_core::LocaleId::new("fr").unwrap()),
    )
    .unwrap();
    let mut values = InterpolationValues::new();
    values.insert("remaining".to_owned(), ScalarValue::from(1_i64));
    let provider = CountingPluralProvider {
        calls: Cell::new(0),
    };
    let DialogueEvent::Line(line) = next_with(
        &asset,
        &mut session,
        &recite_runtime::EmptyDialogueContext,
        LocaleResolution::new()
            .with_provider(&provider)
            .with_values(&values),
    )
    .unwrap() else {
        panic!("expected plural line");
    };
    assert_eq!(provider.calls.get(), 1);
    assert_eq!(line.text, "Vous avez une lettre.");
}

#[test]
fn plural_provider_receives_count_and_translates_selected_form() {
    let asset = plural_asset();
    let mut session = start_scene_with_options(
        &asset,
        None,
        DialogueSessionOptions::new().with_locale(recite_core::LocaleId::new("fr-FR").unwrap()),
    )
    .unwrap();
    let mut values = InterpolationValues::new();
    values.insert("remaining".to_owned(), ScalarValue::from(1_i64));
    let trace = DialogueTrace::new();
    let DialogueEvent::Line(line) = next_with(
        &asset,
        &mut session,
        &recite_runtime::EmptyDialogueContext,
        LocaleResolution::new()
            .with_provider(&PluralProvider)
            .with_values(&values)
            .with_trace(&trace),
    )
    .unwrap() else {
        panic!("expected plural line");
    };
    assert_eq!(line.source_text, "You have one letter.");
    assert_eq!(line.text, "Vous avez une lettre.");
}

#[test]
fn plural_count_rejects_missing_wrong_type_and_negative_values() {
    let asset = plural_asset();
    for value in [ScalarValue::from("two"), ScalarValue::from(-1_i64)] {
        let mut session = start_scene(&asset, None).unwrap();
        let mut values = InterpolationValues::new();
        values.insert("remaining".to_owned(), value);
        let error = next_with(
            &asset,
            &mut session,
            &recite_runtime::EmptyDialogueContext,
            LocaleResolution::new().with_values(&values),
        )
        .expect_err("invalid count is rejected");
        assert!(matches!(
            error,
            recite_runtime::DialogueError::InvalidPluralCount { .. }
        ));
    }
}

impl InterpolationValueProvider for CountingValues {
    fn lookup_value(&self, name: &str) -> Result<Option<ScalarValue>, LocaleError> {
        self.count.set(self.count.get() + 1);
        Ok((name == "display").then(|| ScalarValue::from("Ada")))
    }
}

#[test]
fn repeated_placeholder_resolves_its_provider_value_once() {
    let asset = asset();
    let mut session = start_scene(&asset, None).unwrap();
    let values = CountingValues {
        count: Cell::new(0),
    };
    let event = next_with(
        &asset,
        &mut session,
        &EmptyDialogueContext,
        LocaleResolution::new().with_values(&values),
    )
    .unwrap();
    let DialogueEvent::Line(line) = event else {
        panic!("expected line");
    };
    assert_eq!(line.text, "Hello, Ada; Ada!");
    assert_eq!(values.count.get(), 1);
}

#[test]
fn legacy_wire_rows_preserve_braced_line_and_choice_text() {
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
    choose(
        &asset,
        &mut session,
        choices[0].id.clone(),
        &EmptyDialogueContext,
    )
    .unwrap_or_else(|error| panic!("legacy choice selects: {error}"));
}
