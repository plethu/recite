use std::cell::Cell;

use recite_compiler::compile::{CompileInput, CompileOptions, compile_inputs_with_schema};
use recite_core::{
    LocaleId,
    compiled::{CompiledAssetId, CompiledDialogue, CompilerVersion, SourceMapId},
    schema::load_schema_manifest_str,
};
use recite_runtime::{
    ConditionValue,
    localisation::{
        LocaleError, LocaleLookupOutcome, LocaleProvider, PluralResolution, TextDomain,
    },
    preview::{
        ConditionAnswer, PreviewCommand, PreviewEvent, PreviewInputs, PreviewOptions,
        PreviewSession,
    },
};

const SHARED_ID: &str = "aaaaaaaaaaaaaaaaaaaa";
const LOCKED_ID: &str = "bbbbbbbbbbbbbbbbbbbb";
const LEAVE_ID: &str = "cccccccccccccccccccc";

struct SwitchingProvider {
    translated: Cell<bool>,
}

impl LocaleProvider for SwitchingProvider {
    fn lookup(
        &self,
        _id: &str,
        _source_text: &str,
        domain: TextDomain,
        _locale: &LocaleId,
        _variant: Option<&str>,
    ) -> Result<Option<String>, LocaleError> {
        Ok(self.translated.get().then(|| {
            match domain {
                TextDomain::Line => "Translated prompt.",
                TextDomain::Choice => "Translated choice.",
                TextDomain::AvailabilityReason => "Translated reason.",
                TextDomain::PresentationLabel => "Translated label.",
            }
            .to_owned()
        }))
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
        Err(LocaleError::new("this fixture contains only singular text"))
    }
}

fn asset() -> Result<CompiledDialogue, String> {
    let schema = load_schema_manifest_str(
        "schema.json",
        r#"{
            "schema_version": 1,
            "conditions": {"ready": {"params": [], "returns": "bool"}},
            "availability_reasons": {
                "aaaaaaaaaaaaaaaaaaaa": {"template": "Not ready.", "params": []}
            }
        }"#,
    );
    assert!(schema.diagnostics.is_empty(), "{:?}", schema.diagnostics);
    let schema = schema
        .schema
        .ok_or_else(|| "valid schema is missing".to_owned())?;
    let report = compile_inputs_with_schema(
        [CompileInput::new(
            "dialogue/lookup-history.recite",
            concat!(
                ":: start default\n",
                "> prompt@aaaaaaaaaaaaaaaaaaaa\n  Prompt.\n",
                "  ? locked@bbbbbbbbbbbbbbbbbbbb requires=(ready()) reason=aaaaaaaaaaaaaaaaaaaa\n",
                "    Locked.\n    -> END\n",
                "  ? leave@cccccccccccccccccccc\n    Leave.\n    -> END\n",
            ),
        )],
        CompileOptions::new(
            CompilerVersion::new("0.0.1").map_err(|error| format!("version: {error:?}"))?,
            CompiledAssetId::new("dialogue/lookup-history.recitec")
                .map_err(|error| format!("asset ID: {error:?}"))?,
            SourceMapId::new("dialogue/lookup-history.map")
                .map_err(|error| format!("source map ID: {error:?}"))?,
            schema.canonical_fingerprint(),
        ),
        &schema,
    )
    .map_err(|error| format!("compile: {error:?}"))?;
    assert!(report.is_ok(), "{:?}", report.diagnostics);
    report
        .asset
        .ok_or_else(|| "compiled asset is missing".to_owned())
        .map(|asset| asset.dialogue)
}

fn resolve_prompt(preview: &mut PreviewSession<'_>, provider: &SwitchingProvider) {
    let inputs = PreviewInputs::new().with_locale_provider(provider);
    let pending = preview.step(inputs);
    let [PreviewEvent::ConditionRequested(request)] = pending.events() else {
        panic!("expected ready query, got {:?}", pending.events());
    };
    assert_eq!(request.query().function(), "ready");
    let output = preview.answer(
        request.id(),
        ConditionAnswer::Value(ConditionValue::Bool(false)),
        inputs,
    );
    let [
        PreviewEvent::ConditionResult { .. },
        PreviewEvent::Prompt(prompt),
    ] = output.events()
    else {
        panic!("expected resolved prompt, got {:?}", output.events());
    };
    assert!(!prompt.choices()[0].availability.is_available);
    assert!(prompt.choices()[1].availability.is_available);
}

#[test]
fn latest_lookup_preserves_domains_history_and_replaces_matches_with_missing_entries()
-> Result<(), String> {
    let asset = asset()?;
    let provider = SwitchingProvider {
        translated: Cell::new(true),
    };
    let mut preview = PreviewSession::new(
        &asset,
        None,
        PreviewOptions::new().with_locale(LocaleId::new("fr").expect("locale")),
    )
    .expect("preview");
    let initial = preview.snapshot().expect("initial snapshot");
    assert!(
        preview
            .trace()
            .latest_localized_lookup(SHARED_ID, TextDomain::Line)
            .is_none()
    );
    resolve_prompt(&mut preview, &provider);
    for (domain, text) in [
        (TextDomain::Line, "Translated prompt."),
        (TextDomain::AvailabilityReason, "Translated reason."),
    ] {
        let lookup = preview
            .trace()
            .latest_localized_lookup(SHARED_ID, domain)
            .expect("recorded lookup in the requested domain");
        assert_eq!(lookup.resolved_text.as_deref(), Some(text));
        assert_eq!(lookup.outcome, LocaleLookupOutcome::Matched);
    }
    assert!(
        preview
            .trace()
            .latest_localized_lookup(SHARED_ID, TextDomain::Choice)
            .is_none()
    );
    let first = preview
        .trace()
        .localized_lookups()
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        first
            .iter()
            .map(|lookup| (lookup.id.as_str(), lookup.domain))
            .collect::<Vec<_>>(),
        [
            (SHARED_ID, TextDomain::Line),
            (SHARED_ID, TextDomain::AvailabilityReason),
            (LOCKED_ID, TextDomain::Choice),
            (LEAVE_ID, TextDomain::Choice),
        ]
    );
    let restarted = preview.dispatch(PreviewCommand::Restart, PreviewInputs::new());
    assert!(matches!(
        restarted.events(),
        [PreviewEvent::Restarted { .. }]
    ));
    provider.translated.set(false);
    resolve_prompt(&mut preview, &provider);
    let history = preview
        .trace()
        .localized_lookups()
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(history.len(), 8);
    assert_eq!(&history[..4], first);
    for (previous, latest) in first.iter().zip(&history[4..]) {
        assert_eq!(latest.id, previous.id);
        assert_eq!(latest.domain, previous.domain);
        assert_eq!(latest.outcome, LocaleLookupOutcome::MissingEntry);
        assert_eq!(latest.resolved_text, None);
        assert_eq!(
            preview
                .trace()
                .latest_localized_lookup(&latest.id, latest.domain),
            Some(latest)
        );
    }
    let cloned = preview.trace().clone();
    assert_eq!(cloned, *preview.trace());
    for latest in &history[4..] {
        assert_eq!(
            cloned.latest_localized_lookup(&latest.id, latest.domain),
            Some(latest)
        );
    }
    let restored = preview.restore(initial).expect("restore initial snapshot");
    assert!(matches!(restored.events(), [PreviewEvent::Restored]));
    assert_eq!(preview.trace().localized_lookups().count(), 0);
    for latest in &history[4..] {
        assert!(
            preview
                .trace()
                .latest_localized_lookup(&latest.id, latest.domain)
                .is_none()
        );
        assert_eq!(
            cloned.latest_localized_lookup(&latest.id, latest.domain),
            Some(latest)
        );
    }
    resolve_prompt(&mut preview, &provider);
    assert_eq!(preview.trace().localized_lookups().count(), 4);
    assert_eq!(
        preview
            .trace()
            .latest_localized_lookup(SHARED_ID, TextDomain::Line)
            .expect("new missing lookup after restore")
            .outcome,
        LocaleLookupOutcome::MissingEntry
    );
    Ok(())
}
