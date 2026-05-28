use std::cell::Cell;
use std::collections::BTreeMap;
use std::fs;

use recite_compiler::{
    CompileInput, CompileOptions, compile_inputs_with_schema, extract_pot_with_schema,
};
use recite_core::{
    CompiledAssetId, CompilerVersion, LocaleId, SchemaFingerprint, SourceMapId,
    load_schema_manifest_str,
};
use recite_fixturegen::{FixtureConfigSet, FixtureProfile, SummarySet, generate_tiny_in_memory};
use recite_runtime::{
    ConditionEvaluationError, ConditionQuery, ConditionValue, DialogueContext, DialogueEvent,
    DialogueSessionOptions, EffectAck, LocaleProvider, TextDomain, acknowledge_effect,
    choose_with_locale_provider, decode_session_messagepack, encode_session_messagepack,
    next_with_locale_provider, start_scene_with_options,
};

fn tiny_profile(seed: u64) -> FixtureProfile {
    FixtureProfile {
        name: "tiny".to_owned(),
        seed,
        blocks: 10,
        lines: 100,
        choices: 20,
        localisable_entries: 120,
        generated_words: 1_000,
        shards: 2,
    }
}

#[test]
fn same_profile_seed_produces_byte_identical_tiny_output() {
    let first = generate_tiny_in_memory(&tiny_profile(72)).expect("first fixture");
    let second = generate_tiny_in_memory(&tiny_profile(72)).expect("second fixture");

    assert_eq!(first.files, second.files);
    assert_eq!(first.summary, second.summary);
}

#[test]
fn changed_seed_changes_summary_hash() {
    let first = generate_tiny_in_memory(&tiny_profile(72)).expect("first fixture");
    let second = generate_tiny_in_memory(&tiny_profile(73)).expect("second fixture");

    assert_ne!(first.summary.summary_hash, second.summary.summary_hash);
}

#[test]
fn checked_in_tiny_fixture_matches_regenerated_output() {
    let generated = generate_tiny_in_memory(&tiny_profile(72)).expect("tiny fixture");
    let synthetic_root = fixture_root();
    let fixture_root = synthetic_root.join("tiny");

    for (path, expected) in &generated.files {
        let actual = fs::read(fixture_root.join(path)).unwrap_or_else(|error| {
            panic!("read checked-in generated file {path}: {error}");
        });
        assert_eq!(&actual, expected, "checked-in {path} drifted");
    }

    let checked_summary: serde_json::Value =
        serde_json::from_slice(&fs::read(synthetic_root.join("summaries/tiny.json")).unwrap())
            .expect("checked-in summary JSON");
    let generated_summary =
        serde_json::to_value(&generated.summary).expect("generated summary JSON");
    assert_eq!(checked_summary, generated_summary);
}

#[test]
fn profile_counts_match_spec_budgets() {
    let profiles_path = fixture_root().join("profiles.toml");
    let profiles = FixtureConfigSet::load_path(&profiles_path).expect("profiles load");
    for expected in [
        ("tiny", 10, 100, 20, 120, 1_000),
        ("small", 100, 1_000, 200, 1_200, 10_000),
        ("medium", 1_000, 10_000, 2_000, 12_000, 100_000),
        ("large", 5_000, 50_000, 10_000, 60_000, 500_000),
        ("epic", 10_000, 80_000, 20_000, 100_000, 1_000_000),
    ] {
        let summary = SummarySet::generate_one(profiles.profile(expected.0).expect("profile"))
            .expect("summary");
        assert_eq!(summary.counts.blocks, expected.1);
        assert_eq!(summary.counts.lines, expected.2);
        assert_eq!(summary.counts.choices, expected.3);
        assert_eq!(summary.counts.localisable_entries, expected.4);
        assert!(summary.counts.generated_words >= expected.5);
    }
}

#[test]
fn generated_tiny_fixture_validates_compiles_extracts_and_traverses() {
    let generated = generate_tiny_in_memory(&tiny_profile(72)).expect("tiny fixture");
    let schema_source = text(&generated.files, "schema/synthetic.schema.json");
    let schema = load_schema_manifest_str("schema/synthetic.schema.json", schema_source)
        .schema
        .expect("schema loads");
    let inputs = source_inputs(&generated.files);
    let options = CompileOptions::new(
        CompilerVersion::new("fixturegen-test").expect("compiler version"),
        CompiledAssetId::new("synthetic-tiny").expect("asset id"),
        SourceMapId::new("synthetic-tiny-map").expect("source map id"),
        SchemaFingerprint::NoSchema,
    );
    let report =
        compile_inputs_with_schema(inputs.clone(), options, &schema).expect("compile succeeds");
    assert!(
        report.diagnostics.is_empty(),
        "compile diagnostics: {:?}",
        report.diagnostics
    );
    let asset = report.asset.expect("compiled asset").dialogue;

    let pot = extract_pot_with_schema(inputs, &schema);
    assert!(
        pot.diagnostics.is_empty(),
        "pot diagnostics: {:?}",
        pot.diagnostics
    );
    assert_eq!(
        pot.catalog.expect("catalog").entries.len(),
        generated.summary.counts.localisable_entries as usize + 9
    );
    let catalog = GeneratedCatalog::parse(text(&generated.files, "locales/en-US.po"));

    let mut session = start_scene_with_options(
        &asset,
        Some("block_00000"),
        DialogueSessionOptions::new().with_locale(LocaleId::new("en-US").expect("locale")),
    )
    .expect("start scene");
    let context = FixtureContext::default();
    let mut traversal = GeneratedTraversal::new(&asset, &mut session, &context, &catalog);
    for _ in 0..80 {
        if traversal.next() {
            break;
        }
    }

    assert!(
        traversal.saw_prompt,
        "runtime traversal should reach a prompt"
    );
    assert!(
        traversal.saw_blocking,
        "runtime traversal should emit a blocking effect"
    );
    assert!(
        context.saw_relationship.get(),
        "runtime traversal should evaluate relationship-style state"
    );
    assert!(
        traversal.saw_localised_line,
        "runtime traversal should use generated locale catalog translations"
    );
}

struct GeneratedTraversal<'a> {
    asset: &'a recite_core::CompiledDialogue,
    session: &'a mut recite_runtime::DialogueSession,
    context: &'a dyn DialogueContext,
    locale_provider: &'a dyn LocaleProvider,
    saw_prompt: bool,
    saw_blocking: bool,
    saw_localised_line: bool,
}

impl<'a> GeneratedTraversal<'a> {
    fn new(
        asset: &'a recite_core::CompiledDialogue,
        session: &'a mut recite_runtime::DialogueSession,
        context: &'a dyn DialogueContext,
        locale_provider: &'a dyn LocaleProvider,
    ) -> Self {
        Self {
            asset,
            session,
            context,
            locale_provider,
            saw_prompt: false,
            saw_blocking: false,
            saw_localised_line: false,
        }
    }

    fn next(&mut self) -> bool {
        let event =
            next_with_locale_provider(self.asset, self.session, self.context, self.locale_provider)
                .expect("next");
        self.handle_event(event)
    }

    fn handle_event(&mut self, event: DialogueEvent) -> bool {
        match event {
            DialogueEvent::Effect(effect) => {
                if matches!(effect.mode, recite_runtime::DialogueEffectMode::Blocking) {
                    self.saw_blocking = true;
                    let bytes =
                        encode_session_messagepack(self.session).expect("encode blocked session");
                    let mut restored = decode_session_messagepack(self.asset, &bytes)
                        .expect("restore blocked session");
                    let restored_effect = match next_with_locale_provider(
                        self.asset,
                        &mut restored,
                        self.context,
                        self.locale_provider,
                    )
                    .expect("reemit restored effect")
                    {
                        DialogueEvent::Effect(effect) => effect,
                        other => panic!("expected restored blocking effect, got {other:?}"),
                    };
                    assert_eq!(restored_effect, effect);
                    acknowledge_effect(&mut restored, effect.id, EffectAck::Completed)
                        .expect("ack");
                    *self.session = restored;
                }
                false
            }
            DialogueEvent::Prompt { choices, .. } => {
                self.saw_prompt = true;
                if let Some(first) = choices.first()
                    && first.text.starts_with("choice translation for ")
                {
                    self.saw_localised_line = true;
                }
                let selected = choices[0].id.clone();
                let event = choose_with_locale_provider(
                    self.asset,
                    self.session,
                    selected,
                    self.context,
                    self.locale_provider,
                )
                .expect("choose");
                match event {
                    DialogueEvent::End { .. } => true,
                    nested => self.handle_event(nested),
                }
            }
            DialogueEvent::End { .. } => true,
            DialogueEvent::Line(_) => false,
        }
    }
}

#[derive(Default)]
struct FixtureContext {
    saw_relationship: Cell<bool>,
}

impl DialogueContext for FixtureContext {
    fn evaluate_condition(
        &self,
        query: ConditionQuery<'_>,
    ) -> Result<ConditionValue, ConditionEvaluationError> {
        match query.function() {
            "flag" | "counter_gte" => Ok(ConditionValue::Bool(true)),
            "relationship" => {
                self.saw_relationship.set(true);
                Ok(ConditionValue::EnumVariant("active".to_owned()))
            }
            function => Err(ConditionEvaluationError::new(format!(
                "unexpected condition `{function}`"
            ))),
        }
    }
}

#[derive(Debug)]
struct GeneratedCatalog {
    entries: BTreeMap<(String, String, TextDomain), String>,
}

impl GeneratedCatalog {
    fn parse(source: &str) -> Self {
        let mut entries = BTreeMap::new();
        let mut context = None::<String>;
        let mut source_text = None::<String>;

        for line in source.lines() {
            if let Some(value) = line.strip_prefix("msgctxt ") {
                context = Some(unquote(value));
            } else if let Some(value) = line.strip_prefix("msgid ") {
                source_text = Some(unquote(value));
            } else if let Some(value) = line.strip_prefix("msgstr ") {
                let context = context.take().expect("context before msgstr");
                let source_text = source_text.take().expect("msgid before msgstr");
                let domain = if context.starts_with("choice_") {
                    TextDomain::Choice
                } else {
                    TextDomain::Line
                };
                entries.insert((context, source_text, domain), unquote(value));
            }
        }

        Self { entries }
    }
}

impl LocaleProvider for GeneratedCatalog {
    fn lookup(
        &self,
        id: &str,
        source_text: &str,
        domain: TextDomain,
        _locale: &LocaleId,
        _variant: Option<&str>,
    ) -> Option<String> {
        self.entries
            .get(&(id.to_owned(), source_text.to_owned(), domain))
            .cloned()
    }
}

fn unquote(value: &str) -> String {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .expect("generated PO uses single-line quoted strings")
        .to_owned()
}

fn fixture_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/synthetic")
}
fn source_inputs(files: &BTreeMap<String, Vec<u8>>) -> Vec<CompileInput> {
    files
        .iter()
        .filter_map(|(path, bytes)| {
            path.strip_prefix("src/").map(|_| {
                CompileInput::new(
                    path.clone(),
                    String::from_utf8(bytes.clone()).expect("utf8"),
                )
            })
        })
        .collect()
}

fn text<'a>(files: &'a BTreeMap<String, Vec<u8>>, path: &str) -> &'a str {
    std::str::from_utf8(files.get(path).expect("file exists")).expect("utf8")
}
