use std::cell::Cell;

use recite_compiler::{CompileInput, CompileOptions, compile_inputs};
use recite_core::{
    CompiledAssetId, CompilerVersion, LocaleId, ScalarValue, SchemaFingerprint, SourceMapId,
};
use recite_runtime::{
    DialogueError, DialogueEvent, DialogueSessionOptions, DialogueTrace, EmptyDialogueContext,
    InterpolationValueProvider, LocaleError, LocaleProvider, LocaleResolution, PluralResolution,
    TextDomain, next_with, start_scene, start_scene_with_options,
};

const PLURAL_ID: &str = "8843fd6f53f020a12b31";

fn plural_asset() -> recite_core::CompiledDialogue {
    let source = concat!(
        ":: start default\n",
        "> letters_001@8843fd6f53f020a12b31 bind=(count:int=$remaining)\n",
        "  You have one letter.\n",
        "  | You have {count} letters.\n",
        "-> END\n",
    );
    compile_inputs(
        [CompileInput::new(
            "dialogue/plural-count-reuse.recite",
            source,
        )],
        CompileOptions::new(
            CompilerVersion::new("0.0.1")
                .unwrap_or_else(|error| panic!("valid compiler version: {error}")),
            CompiledAssetId::new("plural-count-reuse")
                .unwrap_or_else(|error| panic!("valid asset ID: {error}")),
            SourceMapId::new("plural-count-reuse-map")
                .unwrap_or_else(|error| panic!("valid source map ID: {error}")),
            SchemaFingerprint::NoSchema,
        ),
    )
    .unwrap_or_else(|error| panic!("plural source compiles: {error:?}"))
    .asset
    .unwrap_or_else(|| panic!("plural source produces an asset"))
    .dialogue
}

struct StatefulCountValues {
    calls: Cell<usize>,
}

impl StatefulCountValues {
    fn calls(&self) -> usize {
        self.calls.get()
    }
}

impl InterpolationValueProvider for StatefulCountValues {
    fn lookup_value(&self, name: &str) -> Result<Option<ScalarValue>, LocaleError> {
        assert_eq!(name, "remaining");
        let call = self.calls.get();
        self.calls.set(call + 1);
        Ok(Some(ScalarValue::Integer(if call == 0 { 2 } else { 99 })))
    }
}

#[test]
fn source_plural_reuses_the_count_selected_for_the_arm_and_trace() {
    let asset = plural_asset();
    let mut session = start_scene(&asset, None).unwrap_or_else(|error| panic!("starts: {error}"));
    let values = StatefulCountValues {
        calls: Cell::new(0),
    };
    let trace = DialogueTrace::new();
    let DialogueEvent::Line(line) = next_with(
        &asset,
        &mut session,
        &EmptyDialogueContext,
        LocaleResolution::new()
            .with_values(&values)
            .with_trace(&trace),
    )
    .unwrap_or_else(|error| panic!("source plural emits: {error}")) else {
        panic!("expected plural line");
    };

    assert_eq!(line.text, "You have 2 letters.");
    assert_eq!(line.plural.as_ref().map(|plural| plural.count), Some(2));
    assert_eq!(trace.plural_line(PLURAL_ID).map(|line| line.count), Some(2));
    assert_eq!(values.calls(), 1);
}

struct TranslatedPluralProvider;

impl LocaleProvider for TranslatedPluralProvider {
    fn lookup(
        &self,
        _id: &str,
        _source_text: &str,
        _domain: TextDomain,
        _locale: &LocaleId,
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
        _locale: &LocaleId,
        _variant: Option<&str>,
    ) -> Result<PluralResolution, LocaleError> {
        Ok(PluralResolution {
            template: Some("Vous avez {count} lettres.".to_owned()),
            selected_arm: Some(1),
            matched_locale: Some("fr-FR".to_owned()),
            matched_context: Some(PLURAL_ID.to_owned()),
            matched_key: Some(PLURAL_ID.to_owned()),
            attempts: Vec::new(),
        })
    }
}

#[test]
fn translated_plural_reuses_the_count_selected_for_the_arm() {
    let asset = plural_asset();
    let locale = LocaleId::new("fr-FR").unwrap_or_else(|error| panic!("valid locale: {error}"));
    let mut session = start_scene_with_options(
        &asset,
        None,
        DialogueSessionOptions::new().with_locale(locale),
    )
    .unwrap_or_else(|error| panic!("starts: {error}"));
    let values = StatefulCountValues {
        calls: Cell::new(0),
    };
    let DialogueEvent::Line(line) = next_with(
        &asset,
        &mut session,
        &EmptyDialogueContext,
        LocaleResolution::new()
            .with_provider(&TranslatedPluralProvider)
            .with_values(&values),
    )
    .unwrap_or_else(|error| panic!("translated plural emits: {error}")) else {
        panic!("expected plural line");
    };

    assert_eq!(line.source_text, "You have {count} letters.");
    assert_eq!(line.text, "Vous avez 2 lettres.");
    assert_eq!(line.plural.as_ref().map(|plural| plural.count), Some(2));
    assert_eq!(values.calls(), 1);
}

#[derive(Clone, Copy)]
enum CountLookupOutcome {
    Missing,
    WrongType,
    ProviderError,
}

struct CountLookupProvider {
    outcome: CountLookupOutcome,
    calls: Cell<usize>,
}

impl InterpolationValueProvider for CountLookupProvider {
    fn lookup_value(&self, name: &str) -> Result<Option<ScalarValue>, LocaleError> {
        assert_eq!(name, "remaining");
        self.calls.set(self.calls.get() + 1);
        match self.outcome {
            CountLookupOutcome::Missing => Ok(None),
            CountLookupOutcome::WrongType => Ok(Some(ScalarValue::from("two"))),
            CountLookupOutcome::ProviderError => Err(LocaleError::new("count lookup failed")),
        }
    }
}

#[test]
fn count_lookup_failures_keep_their_existing_errors_and_single_call() {
    for (outcome, expected) in [
        (
            CountLookupOutcome::Missing,
            DialogueError::MissingInterpolationValue {
                name: "remaining".to_owned(),
            },
        ),
        (
            CountLookupOutcome::WrongType,
            DialogueError::InvalidPluralCount {
                name: "remaining".to_owned(),
                reason: "expected int, got string".to_owned(),
            },
        ),
        (
            CountLookupOutcome::ProviderError,
            DialogueError::InterpolationValueFailed {
                name: "remaining".to_owned(),
                reason: "count lookup failed".to_owned(),
            },
        ),
    ] {
        let asset = plural_asset();
        let mut session =
            start_scene(&asset, None).unwrap_or_else(|error| panic!("starts: {error}"));
        let values = CountLookupProvider {
            outcome,
            calls: Cell::new(0),
        };
        let error = next_with(
            &asset,
            &mut session,
            &EmptyDialogueContext,
            LocaleResolution::new().with_values(&values),
        )
        .expect_err("count lookup failure is returned");

        assert_eq!(error, expected);
        assert_eq!(values.calls.get(), 1);
    }
}
