use recite_compiler::{CompileInput, CompileOptions, compile_inputs};
use recite_core::{CompiledAssetId, CompilerVersion, ScalarValue, SchemaFingerprint, SourceMapId};
use recite_runtime::{
    DialogueEvent, DialogueSessionOptions, EmptyDialogueContext, InterpolationValues, LocaleError,
    LocaleProvider, LocaleResolution, PluralResolution, TextDomain, next_with, start_scene,
    start_scene_with_options,
};

fn distinct_plural_asset() -> recite_core::CompiledDialogue {
    let source = concat!(
        ":: start default\n",
        "> letters_001@8843fd6f53f020a12b31 bind=(count:int=$remaining) bind=(name:string=$name)\n",
        "  {name} has one letter.\n",
        "  | You have {count} letters.\n",
        "-> END\n",
    );
    compile_inputs(
        [CompileInput::new("dialogue/distinct-plural.recite", source)],
        CompileOptions::new(
            CompilerVersion::new("0.0.1")
                .unwrap_or_else(|error| panic!("valid compiler version: {error}")),
            CompiledAssetId::new("distinct-plural")
                .unwrap_or_else(|error| panic!("valid asset id: {error}")),
            SourceMapId::new("distinct-plural-map")
                .unwrap_or_else(|error| panic!("valid source map id: {error}")),
            SchemaFingerprint::NoSchema,
        ),
    )
    .unwrap_or_else(|error| panic!("distinct plural source compiles: {error:?}"))
    .asset
    .unwrap_or_else(|| panic!("distinct plural source produces an asset"))
    .dialogue
}

#[test]
fn plural_rendering_only_resolves_bindings_in_the_selected_source_form() {
    let asset = distinct_plural_asset();

    let mut plural_session = start_scene(&asset, None).unwrap();
    let mut plural_values = InterpolationValues::new();
    plural_values.insert("remaining".to_owned(), ScalarValue::from(2_i64));
    let DialogueEvent::Line(line) = next_with(
        &asset,
        &mut plural_session,
        &EmptyDialogueContext,
        LocaleResolution::new().with_values(&plural_values),
    )
    .unwrap() else {
        panic!("expected plural line");
    };
    assert_eq!(line.source_text, "You have {count} letters.");
    assert_eq!(line.text, "You have 2 letters.");

    let mut singular_session = start_scene(&asset, None).unwrap();
    let mut singular_values = InterpolationValues::new();
    singular_values.insert("remaining".to_owned(), ScalarValue::from(1_i64));
    let error = next_with(
        &asset,
        &mut singular_session,
        &EmptyDialogueContext,
        LocaleResolution::new().with_values(&singular_values),
    )
    .expect_err("selected singular binding is required");
    assert!(matches!(
        error,
        recite_runtime::DialogueError::MissingInterpolationValue { ref name }
            if name == "name"
    ));

    let mut wrong_type_session = start_scene(&asset, None).unwrap();
    let mut wrong_type_values = InterpolationValues::new();
    wrong_type_values.insert("remaining".to_owned(), ScalarValue::from(2_i64));
    wrong_type_values.insert("name".to_owned(), ScalarValue::from(42_i64));
    next_with(
        &asset,
        &mut wrong_type_session,
        &EmptyDialogueContext,
        LocaleResolution::new().with_values(&wrong_type_values),
    )
    .expect("wrong type in unselected singular form is ignored");
}

struct TranslatedPluralProvider;

impl LocaleProvider for TranslatedPluralProvider {
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
                "{name} a une lettre.".to_owned()
            } else {
                "Vous avez {count} lettres.".to_owned()
            }),
            selected_arm: Some(usize::from(count != 1)),
            matched_locale: Some("fr".to_owned()),
            matched_context: Some("8843fd6f53f020a12b31".to_owned()),
            matched_key: Some("8843fd6f53f020a12b31".to_owned()),
            attempts: Vec::new(),
        })
    }
}

#[test]
fn translated_plural_rendering_only_resolves_bindings_in_the_selected_template() {
    let asset = distinct_plural_asset();
    let mut session = start_scene_with_options(
        &asset,
        None,
        DialogueSessionOptions::new().with_locale(recite_core::LocaleId::new("fr").unwrap()),
    )
    .unwrap();
    let mut values = InterpolationValues::new();
    values.insert("remaining".to_owned(), ScalarValue::from(2_i64));

    let DialogueEvent::Line(line) = next_with(
        &asset,
        &mut session,
        &EmptyDialogueContext,
        LocaleResolution::new()
            .with_provider(&TranslatedPluralProvider)
            .with_values(&values),
    )
    .unwrap() else {
        panic!("expected plural line");
    };
    assert_eq!(line.source_text, "You have {count} letters.");
    assert_eq!(line.text, "Vous avez 2 lettres.");
}
