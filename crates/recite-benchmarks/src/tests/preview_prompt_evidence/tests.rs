use recite_core::{CompiledAssetId, CompilerVersion, ScalarValue, SchemaFingerprint, SourceMapId};
use recite_runtime::{
    InterpolationValues, LocaleError, LocaleProvider, PluralResolution, PreviewEvent,
    PreviewInputs, PreviewOptions, PreviewPrompt, PreviewSession, TextDomain,
};

use super::digest;

struct PluralArmCountProvider {
    arm_count: usize,
}

impl LocaleProvider for PluralArmCountProvider {
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
        Ok(PluralResolution {
            template: Some("Many {count} things.".to_owned()),
            selected_arm: Some(1),
            matched_locale: Some("test".to_owned()),
            matched_context: None,
            matched_key: Some("prompt".to_owned()),
            attempts: Vec::new(),
        })
    }

    fn validated_plural_arm_count(
        &self,
        _resolution: &PluralResolution,
    ) -> Result<Option<usize>, LocaleError> {
        Ok(Some(self.arm_count))
    }
}

fn plural_prompt_asset() -> recite_core::CompiledDialogue {
    let report = recite_compiler::compile_inputs(
        [recite_compiler::CompileInput::new(
            "prompt.recite",
            concat!(
                ":: start default\n",
                "> prompt@12345678901234567890 bind=(count:int=$count)\n",
                "  One thing.\n",
                "  | Many {count} things.\n",
                "  ? keep@12345678901234567891\n",
                "    Keep.\n",
                "    -> END\n",
            ),
        )],
        recite_compiler::CompileOptions::new(
            CompilerVersion::new("0.0.1").expect("test compiler version is valid"),
            CompiledAssetId::new("dialogue/preview-prompt-hash.recitec")
                .expect("test asset ID is valid"),
            SourceMapId::new("dialogue/preview-prompt-hash.map")
                .expect("test source map ID is valid"),
            SchemaFingerprint::NoSchema,
        ),
    )
    .expect("test prompt asset compiles");
    assert!(
        report.diagnostics.is_empty(),
        "diagnostics: {:?}",
        report.diagnostics
    );
    report.asset.expect("test prompt asset is present").dialogue
}

fn prompt_with_plural_arm_count(
    asset: &recite_core::CompiledDialogue,
    arm_count: usize,
) -> PreviewPrompt {
    let mut values = InterpolationValues::new();
    values.insert("count".to_owned(), ScalarValue::from(2_i64));
    let options = PreviewOptions::new()
        .with_locale(recite_core::LocaleId::new("test").expect("test locale is valid"));
    let provider = PluralArmCountProvider { arm_count };
    let mut preview = PreviewSession::new(asset, None, options).expect("test preview starts");
    let output = preview.step(
        PreviewInputs::new()
            .with_locale_provider(&provider)
            .with_interpolation_values(&values),
    );
    match output.events() {
        [PreviewEvent::Prompt(prompt)] => prompt.clone(),
        events => panic!("expected one plural prompt, got {events:?}"),
    }
}

#[test]
fn plural_arm_count_changes_prompt_evidence_independently() {
    let asset = plural_prompt_asset();
    let two_arm_prompt = prompt_with_plural_arm_count(&asset, 2);
    let three_arm_prompt = prompt_with_plural_arm_count(&asset, 3);

    assert_eq!(two_arm_prompt.identity(), three_arm_prompt.identity());
    assert_eq!(two_arm_prompt.line(), three_arm_prompt.line());
    assert_eq!(two_arm_prompt.choices(), three_arm_prompt.choices());
    assert_ne!(two_arm_prompt, three_arm_prompt);
    assert_ne!(
        digest(&PreviewEvent::Prompt(two_arm_prompt)),
        digest(&PreviewEvent::Prompt(three_arm_prompt)),
    );
}
