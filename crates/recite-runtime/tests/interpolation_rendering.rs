use std::cell::RefCell;

use recite_compiler::compile::{CompileInput, CompileOptions, compile_inputs};
use recite_core::{
    LocaleId, ScalarValue,
    compiled::{
        CompiledAssetId, CompiledDialogue, CompilerVersion, SchemaFingerprint, SourceMapId,
    },
};
use recite_runtime::{
    DialogueError, DialogueEvent, DialogueSessionOptions, EmptyDialogueContext, LocaleResolution,
    localisation::{
        InterpolationValueProvider, LocaleError, LocaleProvider, PluralResolution, TextDomain,
    },
    next_with, start_scene, start_scene_with_options,
};

fn asset() -> CompiledDialogue {
    let source = concat!(
        ":: start default\n",
        "> render_001@8843fd6f53f020a12b31 ",
        "bind=(word:string=$display) bind=(number:int=$integer) ",
        "bind=(decimal:float=$fraction) bind=(flag:bool=$enabled)\n",
        "  Départ \\字 \\{brace\\} {word} / {number} / {decimal} / {flag} / {word} \\\n",
        "-> END\n",
    );
    compile_inputs(
        [CompileInput::new("dialogue/rendering.recite", source)],
        CompileOptions::new(
            CompilerVersion::new("0.0.1")
                .unwrap_or_else(|error| panic!("valid compiler version: {error}")),
            CompiledAssetId::new("rendering")
                .unwrap_or_else(|error| panic!("valid asset ID: {error}")),
            SourceMapId::new("rendering-map")
                .unwrap_or_else(|error| panic!("valid source map ID: {error}")),
            SchemaFingerprint::NoSchema,
        ),
    )
    .unwrap_or_else(|error| panic!("rendering source compiles: {error:?}"))
    .asset
    .unwrap_or_else(|| panic!("rendering source emits an asset"))
    .dialogue
}

struct Values {
    calls: RefCell<Vec<String>>,
    fail: Option<&'static str>,
}

impl Values {
    fn new(fail: Option<&'static str>) -> Self {
        Self {
            calls: RefCell::new(Vec::new()),
            fail,
        }
    }
}

impl InterpolationValueProvider for Values {
    fn lookup_value(&self, name: &str) -> Result<Option<ScalarValue>, LocaleError> {
        self.calls.borrow_mut().push(name.to_owned());
        if self.fail == Some(name) {
            return Err(LocaleError::new("provider unavailable"));
        }
        Ok(Some(match name {
            "display" => ScalarValue::from("🦊 {number} \\{literal\\}"),
            "integer" => ScalarValue::Integer(-42),
            "fraction" => ScalarValue::Float(1.25),
            "enabled" => ScalarValue::Boolean(true),
            _ => panic!("unexpected interpolation value {name}"),
        }))
    }
}

struct Translation;

impl LocaleProvider for Translation {
    fn lookup(
        &self,
        _id: &str,
        _source_text: &str,
        _domain: TextDomain,
        _locale: &LocaleId,
        _variant: Option<&str>,
    ) -> Result<Option<String>, LocaleError> {
        Ok(Some(
            "後\\字 \\{brace\\}\r\n{flag}|{word}|{decimal}|{number}|{word}\\".to_owned(),
        ))
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
        panic!("ordinary line must not resolve a plural");
    }
}

#[test]
fn repeated_values_keep_unicode_escapes_and_scalar_formatting() {
    let asset = asset();
    let values = Values::new(None);
    let mut session = start_scene(&asset, None).unwrap();
    let DialogueEvent::Line(line) = next_with(
        &asset,
        &mut session,
        &EmptyDialogueContext,
        LocaleResolution::new().with_values(&values),
    )
    .unwrap() else {
        panic!("expected interpolated line");
    };
    assert_eq!(
        line.text,
        "Départ \\字 {brace} 🦊 {number} \\{literal\\} / -42 / 1.25 / true / 🦊 {number} \\{literal\\} \\"
    );
    assert_eq!(
        values.calls.borrow().as_slice(),
        ["display", "integer", "fraction", "enabled"]
    );
}

#[test]
fn translated_placeholder_order_preserves_binding_lookup_order_and_crlf() {
    let asset = asset();
    let values = Values::new(None);
    let mut session = start_scene_with_options(
        &asset,
        None,
        DialogueSessionOptions::new().with_locale(LocaleId::new("ja").unwrap()),
    )
    .unwrap();
    let DialogueEvent::Line(line) = next_with(
        &asset,
        &mut session,
        &EmptyDialogueContext,
        LocaleResolution::new()
            .with_values(&values)
            .with_provider(&Translation),
    )
    .unwrap() else {
        panic!("expected translated line");
    };
    assert_eq!(
        line.text,
        "後\\字 {brace}\r\ntrue|🦊 {number} \\{literal\\}|1.25|-42|🦊 {number} \\{literal\\}\\"
    );
    assert_eq!(
        values.calls.borrow().as_slice(),
        ["display", "integer", "fraction", "enabled"]
    );
}

#[test]
fn provider_failure_stops_in_binding_order_before_rendering() {
    let asset = asset();
    let values = Values::new(Some("fraction"));
    let mut session = start_scene(&asset, None).unwrap();
    let error = next_with(
        &asset,
        &mut session,
        &EmptyDialogueContext,
        LocaleResolution::new().with_values(&values),
    )
    .unwrap_err();
    assert_eq!(
        error,
        DialogueError::InterpolationValueFailed {
            name: "fraction".to_owned(),
            reason: "provider unavailable".to_owned(),
        }
    );
    assert_eq!(
        values.calls.borrow().as_slice(),
        ["display", "integer", "fraction"]
    );
}
