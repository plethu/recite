use std::cell::Cell;

#[path = "support/preview.rs"]
mod preview_support;

use preview_support::asset;
use recite_core::{LocaleId, ScalarValue};
use recite_runtime::{
    InterpolationValues, LocaleError, LocaleLookupAttempt, LocaleLookupOutcome,
    LocaleLookupProvenance, LocaleProvider, PluralResolution, PluralResolutionAttempt,
    PluralResolutionOutcome, PreviewError, PreviewEvent, PreviewInputs, PreviewOptions,
    PreviewSession, TextDomain,
};

struct FallbackProvider {
    calls: Cell<usize>,
    malformed: bool,
}

impl LocaleProvider for FallbackProvider {
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

    fn lookup_with_provenance(
        &self,
        id: &str,
        _source_text: &str,
        _domain: TextDomain,
        locale: &LocaleId,
        variant: Option<&str>,
    ) -> Result<LocaleLookupProvenance, LocaleError> {
        self.calls.set(self.calls.get() + 1);
        let context = variant.unwrap_or("");
        let attempts = vec![
            LocaleLookupAttempt::new(
                locale.as_str(),
                context,
                id,
                LocaleLookupOutcome::MissingEntry,
            ),
            LocaleLookupAttempt::new("en-GB", "", id, LocaleLookupOutcome::Matched),
        ];
        let template = if self.malformed {
            Some("Broken {name".to_owned())
        } else {
            Some("Fallback, {name}.".to_owned())
        };
        Ok(LocaleLookupProvenance::new(template)
            .with_match("en-GB", "", id)
            .with_attempts(attempts))
    }

    fn resolve_plural(
        &self,
        id: &str,
        _source_singular: &str,
        _source_plural: &str,
        _count: i64,
        _domain: TextDomain,
        locale: &LocaleId,
        variant: Option<&str>,
    ) -> Result<PluralResolution, LocaleError> {
        self.calls.set(self.calls.get() + 1);
        Ok(PluralResolution {
            template: None,
            selected_arm: None,
            matched_locale: None,
            matched_context: None,
            matched_key: None,
            attempts: vec![
                PluralResolutionAttempt {
                    locale: locale.as_str().to_owned(),
                    context: variant.unwrap_or("").to_owned(),
                    key: id.to_owned(),
                    selected_arm: None,
                    outcome: PluralResolutionOutcome::MissingEntry,
                },
                PluralResolutionAttempt {
                    locale: "en-GB".to_owned(),
                    context: String::new(),
                    key: id.to_owned(),
                    selected_arm: None,
                    outcome: PluralResolutionOutcome::MissingEntry,
                },
            ],
        })
    }
}

#[test]
fn singular_fallback_preserves_real_ordered_attempts() {
    let asset = asset(
        ":: start default\n> hello@12345678901234567890 bind=(name:string=$name)\n  Hello, {name}.\n-> END\n",
    );
    let locale = LocaleId::new("fr-FR").expect("locale");
    let provider = FallbackProvider {
        calls: Cell::new(0),
        malformed: false,
    };
    let mut values = InterpolationValues::new();
    values.insert("name".to_owned(), ScalarValue::from("Ada"));
    let mut preview = PreviewSession::new(
        &asset,
        None,
        PreviewOptions::new()
            .with_locale(locale)
            .with_variant("formal"),
    )
    .expect("preview");
    let output = preview.step(
        PreviewInputs::new()
            .with_locale_provider(&provider)
            .with_interpolation_values(&values),
    );
    assert!(matches!(output.events(), [PreviewEvent::Line(line)] if line.text == "Fallback, Ada."));
    let lookup = preview
        .trace()
        .localized_lookups()
        .next()
        .expect("lookup trace");
    assert_eq!(
        lookup.attempts[0].outcome,
        LocaleLookupOutcome::MissingEntry
    );
    assert_eq!(lookup.attempts[1].locale, "en-GB");
    assert_eq!(lookup.matched_locale.as_deref(), Some("en-GB"));
}

#[test]
fn plural_source_fallback_preserves_repeated_attempt_occurrences() {
    let asset = asset(concat!(
        ":: start default\n",
        "> letters@12345678901234567890 bind=(count:int=$count)\n",
        "  One letter.\n",
        "  | {count} letters.\n",
        "-> END\n",
    ));
    let locale = LocaleId::new("fr-FR").expect("locale");
    let provider = FallbackProvider {
        calls: Cell::new(0),
        malformed: false,
    };
    let mut values = InterpolationValues::new();
    values.insert("count".to_owned(), ScalarValue::from(2_i64));
    let mut preview = PreviewSession::new(&asset, None, PreviewOptions::new().with_locale(locale))
        .expect("preview");
    for _ in 0..2 {
        preview.step(
            PreviewInputs::new()
                .with_locale_provider(&provider)
                .with_interpolation_values(&values),
        );
        preview.dispatch(
            recite_runtime::PreviewCommand::Restart,
            PreviewInputs::new(),
        );
    }
    let occurrences: Vec<_> = preview.trace().plural_lines().collect();
    assert_eq!(occurrences.len(), 2);
    assert_eq!(occurrences[0].0, occurrences[1].0);
    assert_eq!(occurrences[0].1.attempts[0].locale, "fr-FR");
    assert_eq!(occurrences[0].1.attempts[1].locale, "en-GB");
}

#[test]
fn malformed_successful_provider_output_rolls_back() {
    let asset = asset(
        ":: start default\n> hello@12345678901234567890 bind=(name:string=$name)\n  Hello, {name}.\n-> END\n",
    );
    let provider = FallbackProvider {
        calls: Cell::new(0),
        malformed: true,
    };
    let mut preview = PreviewSession::new(
        &asset,
        None,
        PreviewOptions::new().with_locale(LocaleId::new("fr-FR").expect("locale")),
    )
    .expect("preview");
    let before = preview.session().clone();
    let output = preview.step(PreviewInputs::new().with_locale_provider(&provider));
    assert!(matches!(
        output.events(),
        [PreviewEvent::Error(PreviewError::Runtime(
            recite_runtime::DialogueError::InvalidInterpolationSyntax { .. }
        ))]
    ));
    assert_eq!(*preview.session(), before);
}
