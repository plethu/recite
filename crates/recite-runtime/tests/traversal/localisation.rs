use super::*;

#[derive(Debug, Default)]
struct RecordingLocaleProvider {
    translations: BTreeMap<(String, TextDomain, String), String>,
    calls: RefCell<Vec<LocaleCall>>,
}

impl RecordingLocaleProvider {
    fn with(
        mut self,
        id: &str,
        domain: TextDomain,
        variant: Option<&str>,
        translation: &str,
    ) -> Self {
        self.translations.insert(
            (lookup_key(id, variant), domain, "en-GB".to_owned()),
            translation.to_owned(),
        );
        self
    }

    fn calls(&self) -> Vec<LocaleCall> {
        self.calls.borrow().clone()
    }
}

impl LocaleProvider for RecordingLocaleProvider {
    fn lookup(
        &self,
        id: &str,
        source_text: &str,
        domain: TextDomain,
        locale: &LocaleId,
        variant: Option<&str>,
    ) -> Option<String> {
        self.calls.borrow_mut().push(LocaleCall {
            id: id.to_owned(),
            source_text: source_text.to_owned(),
            domain,
            locale: locale.as_str().to_owned(),
            variant: variant.map(str::to_owned),
        });

        variant
            .and_then(|variant| {
                self.translations.get(&(
                    lookup_key(id, Some(variant)),
                    domain,
                    locale.as_str().to_owned(),
                ))
            })
            .or_else(|| {
                self.translations
                    .get(&(lookup_key(id, None), domain, locale.as_str().to_owned()))
            })
            .cloned()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LocaleCall {
    id: String,
    source_text: String,
    domain: TextDomain,
    locale: String,
    variant: Option<String>,
}

fn locale_resolution(provider: &dyn LocaleProvider) -> LocaleResolution<'_> {
    LocaleResolution::new().with_provider(provider)
}

fn variant_locale_resolution<'a>(
    provider: &'a dyn LocaleProvider,
    variant: &'a str,
) -> LocaleResolution<'a> {
    LocaleResolution::new()
        .with_provider(provider)
        .with_variant(variant)
}

#[test]
fn locale_provider_receives_line_lookup_fields_and_variant() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> intro_001\n",
            "  Hello.\n",
            "-> END\n",
        ),
    );
    let provider = RecordingLocaleProvider::default().with(
        "intro_001",
        TextDomain::Line,
        Some("formal"),
        "Bonjour.",
    );
    let mut session = start_scene_with_options(
        &asset,
        None,
        DialogueSessionOptions::new().with_locale(locale("en-GB")),
    )
    .expect("starts");

    let DialogueEvent::Line(line) = runtime_next_with(
        &asset,
        &mut session,
        &EmptyDialogueContext,
        variant_locale_resolution(&provider, "formal"),
    )
    .expect("emits translated line") else {
        panic!("expected line");
    };

    assert_eq!(line.source_text, "Hello.");
    assert_eq!(line.text, "Bonjour.");
    assert_eq!(
        provider.calls(),
        [LocaleCall {
            id: "intro_001".to_owned(),
            source_text: "Hello.".to_owned(),
            domain: TextDomain::Line,
            locale: "en-GB".to_owned(),
            variant: Some("formal".to_owned()),
        }]
    );
}

#[test]
fn variant_lookup_can_fall_back_to_non_variant_translation() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> intro_001\n",
            "  Hello.\n",
            "-> END\n",
        ),
    );
    let provider =
        RecordingLocaleProvider::default().with("intro_001", TextDomain::Line, None, "Salut.");
    let mut session = start_scene_with_options(
        &asset,
        None,
        DialogueSessionOptions::new().with_locale(locale("en-GB")),
    )
    .expect("starts");

    let DialogueEvent::Line(line) = runtime_next_with(
        &asset,
        &mut session,
        &EmptyDialogueContext,
        variant_locale_resolution(&provider, "formal"),
    )
    .expect("emits translated line") else {
        panic!("expected line");
    };

    assert_eq!(line.text, "Salut.");
    assert_eq!(provider.calls()[0].variant.as_deref(), Some("formal"));
}

#[test]
fn missing_translation_falls_back_to_source_text() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> intro_001\n",
            "  Hello.\n",
            "-> END\n",
        ),
    );
    let provider = RecordingLocaleProvider::default();
    let mut session = start_scene_with_options(
        &asset,
        None,
        DialogueSessionOptions::new().with_locale(locale("en-GB")),
    )
    .expect("starts");

    assert_line(
        runtime_next_with(
            &asset,
            &mut session,
            &EmptyDialogueContext,
            locale_resolution(&provider),
        ),
        "intro_001",
        "Hello.",
    );
    assert_eq!(provider.calls()[0].variant, None);
}

#[test]
fn prompt_line_and_choices_are_localised_with_distinct_domains() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_001\n",
            "  What next?\n",
            "  ? ask_work\n",
            "    Ask about work.\n",
            "    -> END\n",
        ),
    );
    let provider = RecordingLocaleProvider::default()
        .with("prompt_001", TextDomain::Line, None, "Que faire ?")
        .with(
            "ask_work",
            TextDomain::Choice,
            Some("formal"),
            "Discuter du travail.",
        );
    let mut session = start_scene_with_options(
        &asset,
        None,
        DialogueSessionOptions::new().with_locale(locale("en-GB")),
    )
    .expect("starts");

    let DialogueEvent::Prompt { line, choices } = runtime_next_with(
        &asset,
        &mut session,
        &EmptyDialogueContext,
        variant_locale_resolution(&provider, "formal"),
    )
    .expect("emits prompt") else {
        panic!("expected prompt");
    };

    assert_eq!(line.expect("prompt line").text, "Que faire ?");
    assert_eq!(choices[0].text, "Discuter du travail.");
    assert_eq!(
        provider
            .calls()
            .iter()
            .map(|call| (&call.id, call.domain, call.variant.as_deref()))
            .collect::<Vec<_>>(),
        [
            (&"prompt_001".to_owned(), TextDomain::Line, Some("formal")),
            (&"ask_work".to_owned(), TextDomain::Choice, Some("formal")),
        ]
    );
}

#[test]
fn availability_reasons_are_localised_and_rendered_with_args() {
    let schema = recite_core::load_schema_manifest_str(
        "fixtures/schema/valid/generated_manifest.json",
        include_str!("../../../../fixtures/schema/valid/generated_manifest.json"),
    )
    .schema
    .expect("valid schema fixture");
    let asset = compile_asset_with_schema(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_001\n",
            "  What next?\n",
            "  ? ask_news requires=(trust_gte(hazel, rhea, 3))\n",
            "    Ask for private news.\n",
            "    -> END\n",
        ),
        &schema,
    );
    let provider = RecordingLocaleProvider::default().with(
        "trust_too_low",
        TextDomain::AvailabilityReason,
        Some("formal"),
        "{subject} ne fait pas assez confiance a {target} ({threshold}).",
    );
    let context = RecordingContext::default().with("trust_gte", false);
    let mut session = start_scene_with_options(
        &asset,
        None,
        DialogueSessionOptions::new().with_locale(locale("en-GB")),
    )
    .expect("starts");

    let DialogueEvent::Prompt { choices, .. } = runtime_next_with(
        &asset,
        &mut session,
        &context,
        variant_locale_resolution(&provider, "formal"),
    )
    .expect("emits prompt") else {
        panic!("expected prompt");
    };

    let Some(ChoiceAvailabilityReasonTree::Reason(reason)) = &choices[0].availability.reason_tree
    else {
        panic!("expected reason tree");
    };
    assert_eq!(
        reason.source_text,
        "{subject} does not trust {target} enough ({threshold})."
    );
    assert_eq!(reason.text, "hazel ne fait pas assez confiance a rhea (3).");
    assert_eq!(
        provider
            .calls()
            .iter()
            .map(|call| (&call.id, call.domain, call.variant.as_deref()))
            .collect::<Vec<_>>(),
        [
            (&"prompt_001".to_owned(), TextDomain::Line, Some("formal")),
            (
                &"trust_too_low".to_owned(),
                TextDomain::AvailabilityReason,
                Some("formal")
            ),
            (&"ask_news".to_owned(), TextDomain::Choice, Some("formal")),
        ]
    );
}

#[test]
fn choosing_prompt_uses_locale_provider_for_followup_line() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_001\n",
            "  What next?\n",
            "  ? continue_001\n",
            "    Continue.\n",
            "    -> next\n",
            ":: next\n",
            "> followup_001\n",
            "  Follow up.\n",
            "-> END\n",
        ),
    );
    let provider = RecordingLocaleProvider::default()
        .with("continue_001", TextDomain::Choice, None, "Continuer.")
        .with("followup_001", TextDomain::Line, Some("formal"), "Suite.");
    let mut session = start_scene_with_options(
        &asset,
        None,
        DialogueSessionOptions::new().with_locale(locale("en-GB")),
    )
    .expect("starts");
    runtime_next_with(
        &asset,
        &mut session,
        &EmptyDialogueContext,
        locale_resolution(&provider),
    )
    .expect("emits prompt");

    let DialogueEvent::Line(line) = runtime_choose_with(
        &asset,
        &mut session,
        ChoiceId::new("continue_001").expect("valid choice id"),
        &EmptyDialogueContext,
        variant_locale_resolution(&provider, "formal"),
    )
    .expect("chooses and emits followup") else {
        panic!("expected line");
    };

    assert_eq!(line.text, "Suite.");
}

fn lookup_key(id: &str, variant: Option<&str>) -> String {
    variant.map_or_else(|| id.to_owned(), |variant| format!("{id}&{variant}"))
}

fn locale(value: &str) -> LocaleId {
    LocaleId::new(value).expect("valid locale")
}
