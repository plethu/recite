use recite_adapter::ReciteDialogueCatalog;
use recite_core::LocaleId;
use recite_runtime::localisation::{
    LocaleLookupAttempt, LocaleLookupOutcome, LocaleProvider, TextDomain,
};

fn locale() -> Result<LocaleId, Box<dyn std::error::Error>> {
    Ok(LocaleId::new("fr-CA")?)
}

#[test]
fn variant_fallback_precedes_exact_locale_base_and_lookup_agrees()
-> Result<(), Box<dyn std::error::Error>> {
    let mut catalog = ReciteDialogueCatalog::new();
    catalog.insert("fr-CA", "line", "Source.", "Exact base.")?;
    catalog.insert_for_domain(
        "fr-CA",
        TextDomain::Line,
        "line",
        "Source.",
        "",
        Some("formal"),
    )?;
    catalog.insert_for_domain(
        "fr",
        TextDomain::Line,
        "line",
        "Source.",
        "Parent variant.",
        Some("formal"),
    )?;
    let locale = locale()?;
    let resolved = catalog.lookup_with_provenance(
        "line",
        "Source.",
        TextDomain::Line,
        &locale,
        Some("formal"),
    )?;
    assert_eq!(resolved.template.as_deref(), Some("Parent variant."));
    assert_eq!(resolved.matched_locale.as_deref(), Some("fr"));
    assert_eq!(resolved.matched_context.as_deref(), Some("line&formal"));
    assert_eq!(resolved.matched_key.as_deref(), Some("line"));
    assert_eq!(
        resolved.attempts,
        vec![
            LocaleLookupAttempt::new(
                "fr-CA",
                "line&formal",
                "line",
                LocaleLookupOutcome::MissingEntry,
            ),
            LocaleLookupAttempt::new("fr", "line&formal", "line", LocaleLookupOutcome::Matched,),
        ]
    );
    assert_eq!(
        catalog.lookup("line", "Source.", TextDomain::Line, &locale, Some("formal"))?,
        resolved.template
    );

    let base = catalog.lookup_with_provenance(
        "line",
        "Source.",
        TextDomain::Line,
        &locale,
        Some("informal"),
    )?;
    assert_eq!(base.template.as_deref(), Some("Exact base."));
    assert_eq!(base.matched_context.as_deref(), Some("line"));
    assert_eq!(
        base.attempts
            .iter()
            .map(|attempt| attempt.outcome.clone())
            .collect::<Vec<_>>(),
        vec![
            LocaleLookupOutcome::MissingEntry,
            LocaleLookupOutcome::MissingEntry,
            LocaleLookupOutcome::Matched,
        ]
    );
    Ok(())
}

#[test]
fn every_singular_domain_reports_canonical_parent_match() -> Result<(), Box<dyn std::error::Error>>
{
    let mut catalog = ReciteDialogueCatalog::new();
    let cases = [
        (TextDomain::Line, "line", "line"),
        (TextDomain::Choice, "choice", "choice"),
        (
            TextDomain::AvailabilityReason,
            "blocked",
            "availability_reason:blocked",
        ),
        (
            TextDomain::PresentationLabel,
            "label",
            "presentation_label:label",
        ),
    ];
    for (domain, id, _) in cases {
        catalog.insert_for_domain("fr", domain, id, "Source.", "Traduit.", None)?;
    }
    for (domain, id, context) in cases {
        let resolved = catalog.lookup_with_provenance(id, "Source.", domain, &locale()?, None)?;
        assert_eq!(resolved.template.as_deref(), Some("Traduit."));
        assert_eq!(resolved.matched_locale.as_deref(), Some("fr"));
        assert_eq!(resolved.matched_context.as_deref(), Some(context));
        assert_eq!(resolved.matched_key.as_deref(), Some(id));
        assert_eq!(
            resolved.attempts,
            vec![
                LocaleLookupAttempt::new("fr-CA", context, id, LocaleLookupOutcome::MissingEntry),
                LocaleLookupAttempt::new("fr", context, id, LocaleLookupOutcome::Matched),
            ]
        );
    }
    Ok(())
}

#[test]
fn terminal_missing_lookup_keeps_attempts_without_inventing_a_match()
-> Result<(), Box<dyn std::error::Error>> {
    let catalog = ReciteDialogueCatalog::new();
    let locale = locale()?;
    let resolved = catalog.lookup_with_provenance(
        "missing",
        "Authored source.",
        TextDomain::Choice,
        &locale,
        Some("formal"),
    )?;
    assert_eq!(resolved.template, None);
    assert_eq!(resolved.matched_locale, None);
    assert_eq!(resolved.matched_context, None);
    assert_eq!(resolved.matched_key, None);
    assert_eq!(
        resolved.attempts,
        vec![
            LocaleLookupAttempt::new(
                "fr-CA",
                "missing&formal",
                "missing",
                LocaleLookupOutcome::MissingEntry,
            ),
            LocaleLookupAttempt::new(
                "fr",
                "missing&formal",
                "missing",
                LocaleLookupOutcome::MissingEntry,
            ),
            LocaleLookupAttempt::new(
                "fr-CA",
                "missing",
                "missing",
                LocaleLookupOutcome::MissingEntry,
            ),
            LocaleLookupAttempt::new(
                "fr",
                "missing",
                "missing",
                LocaleLookupOutcome::MissingEntry
            ),
        ]
    );
    assert_eq!(
        catalog.lookup(
            "missing",
            "Authored source.",
            TextDomain::Choice,
            &locale,
            Some("formal")
        )?,
        None
    );
    Ok(())
}
