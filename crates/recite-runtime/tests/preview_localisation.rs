#[path = "support/preview.rs"]
mod preview_support;

use std::cell::Cell;

use preview_support::asset;
use recite_core::{LocaleId, ScalarValue};
use recite_runtime::{
    ConditionAnswer, ConditionValue, InterpolationValues, LocaleError, LocaleLookupAttempt,
    LocaleLookupOutcome, LocaleLookupProvenance, LocaleProvider, PluralResolution, PreviewError,
    PreviewEvent, PreviewInputs, PreviewOptions, PreviewSession, TextDomain,
};

struct FrenchProvider {
    calls: Cell<usize>,
    fail_lookup: bool,
}

impl LocaleProvider for FrenchProvider {
    fn lookup(
        &self,
        _id: &str,
        _source_text: &str,
        _domain: TextDomain,
        _locale: &LocaleId,
        _variant: Option<&str>,
    ) -> Result<Option<String>, LocaleError> {
        self.calls.set(self.calls.get() + 1);
        if self.fail_lookup {
            return Err(LocaleError::new("catalogue unavailable"));
        }
        Ok(Some("Bonjour, {name}.".to_owned()))
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
        self.calls.set(self.calls.get() + 1);
        Ok(PluralResolution {
            template: Some("Vous avez {count} lettres.".to_owned()),
            selected_arm: Some(1),
            matched_locale: Some("fr-FR".to_owned()),
            matched_context: Some("12345678901234567891".to_owned()),
            matched_key: Some("12345678901234567891".to_owned()),
            attempts: Vec::new(),
        })
    }

    fn lookup_with_provenance(
        &self,
        id: &str,
        _source_text: &str,
        domain: TextDomain,
        locale: &LocaleId,
        variant: Option<&str>,
    ) -> Result<LocaleLookupProvenance, LocaleError> {
        self.calls.set(self.calls.get() + 1);
        if self.fail_lookup {
            return Err(LocaleError::new("catalogue unavailable"));
        }
        Ok(
            LocaleLookupProvenance::new(Some(if domain == TextDomain::Choice {
                "Choisir.".to_owned()
            } else {
                "Bonjour, {name}.".to_owned()
            }))
            .with_match(locale.as_str(), variant.unwrap_or(""), id)
            .with_attempts(vec![LocaleLookupAttempt::new(
                locale.as_str(),
                variant.unwrap_or(""),
                id,
                LocaleLookupOutcome::Matched,
            )]),
        )
    }
}

#[test]
fn explicit_locale_keeps_source_interpolation_and_plural_provenance() {
    let asset = asset(concat!(
        ":: start default\n",
        "> hello@12345678901234567890 bind=(name:string=$display)\n",
        "  Hello, {name}.\n",
        "> letters@12345678901234567891 bind=(count:int=$remaining)\n",
        "  You have one letter.\n",
        "  | You have {count} letters.\n",
        "-> END\n",
    ));
    let locale = LocaleId::new("fr-FR").expect("locale");
    let provider = FrenchProvider {
        calls: Cell::new(0),
        fail_lookup: false,
    };
    let mut values = InterpolationValues::new();
    values.insert("display".to_owned(), ScalarValue::from("Ada"));
    values.insert("remaining".to_owned(), ScalarValue::from(2_i64));
    let options = PreviewOptions::new()
        .with_locale(locale)
        .with_variant("formal");
    let mut preview = PreviewSession::new(&asset, None, options).expect("start");

    let first = preview.step(
        PreviewInputs::new()
            .with_locale_provider(&provider)
            .with_interpolation_values(&values),
    );
    let PreviewEvent::Line(line) = &first.events()[0] else {
        panic!("expected localized line: {:?}", first.events());
    };
    assert_eq!(line.source_text, "Hello, {name}.");
    assert_eq!(line.text, "Bonjour, Ada.");

    let second = preview.step(
        PreviewInputs::new()
            .with_locale_provider(&provider)
            .with_interpolation_values(&values),
    );
    let PreviewEvent::Line(line) = &second.events()[0] else {
        panic!("expected localized plural: {:?}", second.events());
    };
    assert_eq!(line.source_text, "You have {count} letters.");
    assert_eq!(line.text, "Vous avez 2 lettres.");
    let plural = line.plural.as_ref().expect("plural provenance");
    assert_eq!(plural.singular_source_text, "You have one letter.");
    assert_eq!(plural.plural_source_text, "You have {count} letters.");
    assert_eq!(plural.count, 2);
    assert_eq!(plural.resolution.matched_locale.as_deref(), Some("fr-FR"));
    assert_eq!(plural.resolution.matched_arm, Some(1));
    assert_eq!(
        preview.trace().locale().map(LocaleId::as_str),
        Some("fr-FR")
    );
    assert_eq!(preview.trace().variant(), Some("formal"));
    assert_eq!(provider.calls.get(), 2);
    let lookups: Vec<_> = preview.trace().localized_lookups().collect();
    assert_eq!(lookups.len(), 1);
    assert_eq!(lookups[0].matched_locale.as_deref(), Some("fr-FR"));
    assert_eq!(lookups[0].matched_context.as_deref(), Some("formal"));
    assert_eq!(
        lookups[0].resolved_text.as_deref(),
        Some("Bonjour, {name}.")
    );
    assert_eq!(lookups[0].attempts[0].outcome, LocaleLookupOutcome::Matched);
}

#[test]
fn locale_provider_failure_leaves_session_unchanged() {
    let asset = asset(
        ":: start default\n> hello@12345678901234567890 bind=(name:string=$display)\n  Hello, {name}.\n-> END\n",
    );
    let locale = LocaleId::new("fr-FR").expect("locale");
    let provider = FrenchProvider {
        calls: Cell::new(0),
        fail_lookup: true,
    };
    let mut preview = PreviewSession::new(&asset, None, PreviewOptions::new().with_locale(locale))
        .expect("start");
    let before = preview.session().clone();
    let output = preview.step(PreviewInputs::new().with_locale_provider(&provider));
    assert!(matches!(
        output.events(),
        [PreviewEvent::Error(PreviewError::Runtime(
            recite_runtime::DialogueError::LocaleLookupFailed { .. }
        ))]
    ));
    assert_eq!(*preview.session(), before);
    assert_eq!(provider.calls.get(), 1);
}

#[test]
fn identical_sessions_have_identical_expected_trace_for_condition_replay() {
    let asset = asset(concat!(
        ":: start default\n",
        ":if trusts(player)\n",
        "  > yes@12345678901234567890\n    Yes.\n",
        ":else\n",
        "  > no@12345678901234567891\n    No.\n",
        "-> END\n",
    ));
    let mut left = PreviewSession::new(&asset, None, PreviewOptions::new()).expect("left");
    let mut right = PreviewSession::new(&asset, None, PreviewOptions::new()).expect("right");
    let left_request = match &left.step(PreviewInputs::new()).events()[0] {
        PreviewEvent::ConditionRequested(request) => request.clone(),
        event => panic!("expected condition request, got {event:?}"),
    };
    let right_request = match &right.step(PreviewInputs::new()).events()[0] {
        PreviewEvent::ConditionRequested(request) => request.clone(),
        event => panic!("expected condition request, got {event:?}"),
    };
    assert_eq!(left_request, right_request);
    let left_line = left.answer(
        left_request.id(),
        ConditionAnswer::Value(ConditionValue::Bool(true)),
        PreviewInputs::new(),
    );
    let right_line = right.answer(
        right_request.id(),
        ConditionAnswer::Value(ConditionValue::Bool(true)),
        PreviewInputs::new(),
    );
    assert!(matches!(
        left_line.events(),
        [PreviewEvent::ConditionResult { .. }, PreviewEvent::Line(_)]
    ));
    assert_eq!(left_line.events(), right_line.events());
    let left_end = left.step(PreviewInputs::new());
    let right_end = right.step(PreviewInputs::new());
    assert!(matches!(left_end.events(), [PreviewEvent::End { .. }]));
    assert_eq!(left_end.events(), right_end.events());
    assert_eq!(left.trace(), right.trace());
    assert_eq!(left.transcript(), right.transcript());
}

#[test]
fn repeated_localized_occurrences_preserve_order_after_restart() {
    let asset = asset(
        ":: start default\n> hello@12345678901234567890 bind=(name:string=$display)\n  Hello, {name}.\n-> END\n",
    );
    let provider = FrenchProvider {
        calls: Cell::new(0),
        fail_lookup: false,
    };
    let mut preview = PreviewSession::new(
        &asset,
        None,
        PreviewOptions::new().with_locale(LocaleId::new("fr-FR").expect("locale")),
    )
    .expect("start");
    let mut values = InterpolationValues::new();
    values.insert("display".to_owned(), ScalarValue::from("Ada"));
    preview.step(
        PreviewInputs::new()
            .with_locale_provider(&provider)
            .with_interpolation_values(&values),
    );
    preview.dispatch(
        recite_runtime::PreviewCommand::Restart,
        PreviewInputs::new(),
    );
    preview.step(
        PreviewInputs::new()
            .with_locale_provider(&provider)
            .with_interpolation_values(&values),
    );
    let occurrences: Vec<_> = preview.trace().localized_lookups().collect();
    assert_eq!(occurrences.len(), 2);
    assert_eq!(occurrences[0].id, occurrences[1].id);
    assert_eq!(provider.calls.get(), 2);
}
