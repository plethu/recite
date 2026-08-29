mod support;

use recite_core::LocaleId;
use recite_core::ScalarValue;
use recite_godot::{ReciteDialogueCatalog, ReciteDialogueDriver, ReciteOutput};
use recite_runtime::{InterpolationValues, LocaleProvider, TextDomain};

use support::{assert_effect, assert_line, compile_asset, must_ok, must_ok_unit, output_kinds};

fn localised_asset() -> recite_godot::ReciteDialogueAsset {
    compile_asset(
        "dialogue/localisation.recite",
        "dialogue/localisation.recitec",
        concat!(
            ":: start default\n",
            "> greeting@73000000000000000001 bind=(name:string=$name)\n",
            "  Hello {name}.\n",
            "> letters@73000000000000000002 bind=(count:int=$count)\n",
            "  You have one letter.\n",
            "  | You have {count} letters.\n",
            "> prompt@73000000000000000003\n",
            "  Pick one.\n",
            "  ? choose@73000000000000000004\n",
            "    Choose this.\n",
            "    -> after\n",
            ":: after\n",
            "! blocking finish()\n",
            "> after@73000000000000000005\n",
            "  Finished.\n",
            "-> END\n",
        ),
    )
}

fn values() -> InterpolationValues {
    InterpolationValues::from([
        ("name".to_owned(), ScalarValue::String("Ada".to_owned())),
        ("count".to_owned(), ScalarValue::Integer(2)),
    ])
}

fn catalog() -> ReciteDialogueCatalog {
    let mut catalog = ReciteDialogueCatalog::new();
    must_ok_unit(catalog.set_plural_forms("fr", "nplurals=2; plural=(n != 1);"));
    must_ok_unit(catalog.insert(
        "fr",
        "73000000000000000001",
        "Hello {name}.",
        "Bonjour {name}.",
    ));
    must_ok_unit(catalog.insert_for_domain(
        "fr",
        TextDomain::Choice,
        "73000000000000000004",
        "Choose this.",
        "Choisir ceci.",
        None,
    ));
    must_ok_unit(catalog.insert_for_domain(
        "fr",
        TextDomain::Line,
        "73000000000000000001",
        "Hello {name}.",
        "Bonjour formel {name}.",
        Some("formal"),
    ));
    must_ok_unit(catalog.insert_for_domain(
        "fr",
        TextDomain::Choice,
        "73000000000000000004",
        "Choose this.",
        "Choisir ceci formel.",
        Some("formal"),
    ));
    must_ok_unit(catalog.insert_plural(
        "fr",
        "73000000000000000002",
        "You have one letter.",
        "You have {count} letters.",
        vec![
            "Vous avez une lettre.".to_owned(),
            "Vous avez {count} lettres.".to_owned(),
        ],
        None,
    ));
    must_ok_unit(catalog.insert_for_domain(
        "fr",
        TextDomain::Line,
        "73000000000000000005",
        "Finished.",
        "Terminé.",
        None,
    ));
    catalog
}

#[test]
fn catalog_translates_line_plural_choice_and_restore() {
    let asset = localised_asset();
    let mut driver = ReciteDialogueDriver::new();
    driver.set_interpolation_values(values());
    driver.set_locale_catalog(catalog());

    let outputs = must_ok(driver.start(&asset, None, Some("fr-CA")));
    assert_eq!(output_kinds(&outputs), ["line", "line", "prompt"]);
    assert_line(&outputs[0], "73000000000000000001", "Bonjour Ada.");
    assert_line(&outputs[1], "73000000000000000002", "Vous avez 2 lettres.");
    let plural = match &outputs[1] {
        ReciteOutput::Line(line) => match line.plural.as_ref() {
            Some(plural) => plural,
            None => panic!("expected plural metadata"),
        },
        output => panic!("expected plural line, got {output:?}"),
    };
    assert_eq!(
        plural.resolution.outcome,
        recite_runtime::DialoguePluralResolutionOutcome::Translated
    );
    assert_eq!(
        plural.resolution.attempts[0].outcome,
        recite_runtime::PluralResolutionOutcome::MissingPluralForms
    );
    assert_eq!(
        plural.resolution.attempts[1].outcome,
        recite_runtime::PluralResolutionOutcome::Matched
    );
    let ReciteOutput::Prompt { choices, .. } = &outputs[2] else {
        panic!("expected prompt output");
    };
    assert_eq!(choices[0].text, "Choisir ceci.");

    let snapshot = driver.snapshot().expect("snapshot at prompt");
    driver.end_session().expect("end active session");

    let mut restored = ReciteDialogueDriver::new();
    restored.set_interpolation_values(values());
    restored.set_locale_catalog(catalog());
    assert!(must_ok(restored.restore(&asset, &snapshot)).is_empty());
    let outputs = must_ok(restored.select_choice("73000000000000000004"));
    assert_eq!(output_kinds(&outputs), ["effect"]);
    let effect_id = assert_effect(&outputs[0], "finish", "blocking");
    let outputs = must_ok(restored.acknowledge_effect(&effect_id, true, None));
    assert_eq!(output_kinds(&outputs), ["line", "end"]);
    assert_line(&outputs[0], "73000000000000000005", "Terminé.");
}

#[test]
fn missing_catalog_entries_fall_back_to_authored_source() {
    let asset = localised_asset();
    let mut driver = ReciteDialogueDriver::new();
    driver.set_interpolation_values(values());
    driver.set_locale_catalog(ReciteDialogueCatalog::new());

    let outputs = must_ok(driver.start(&asset, None, Some("fr-CA")));
    assert_line(&outputs[0], "73000000000000000001", "Hello Ada.");
    assert_line(&outputs[1], "73000000000000000002", "You have 2 letters.");
}

#[test]
fn variant_is_context_first_and_re_supplied_on_restore() {
    let asset = localised_asset();
    let mut driver = ReciteDialogueDriver::new();
    driver.set_interpolation_values(values());
    driver.set_locale_catalog(catalog());

    let outputs = must_ok(driver.start_with_variant(&asset, None, Some("fr-CA"), Some("formal")));
    assert_line(&outputs[0], "73000000000000000001", "Bonjour formel Ada.");
    let ReciteOutput::Prompt { choices, .. } = &outputs[2] else {
        panic!("expected prompt output");
    };
    assert_eq!(choices[0].text, "Choisir ceci formel.");

    let snapshot = driver.snapshot().expect("snapshot at prompt");
    driver.end_session().expect("end active session");
    let mut restored = ReciteDialogueDriver::new();
    restored.set_interpolation_values(values());
    restored.set_locale_catalog(catalog());
    assert!(must_ok(restored.restore_with_variant(&asset, &snapshot, Some("formal"))).is_empty());
}

#[test]
fn catalog_exposes_reason_and_label_domains_and_rejects_reachable_plural_arms() {
    let mut catalog = ReciteDialogueCatalog::new();
    must_ok_unit(catalog.insert_for_domain(
        "fr",
        TextDomain::AvailabilityReason,
        "locked",
        "Requires a key.",
        "Nécessite une clé.",
        None,
    ));
    must_ok_unit(catalog.insert_for_domain(
        "fr",
        TextDomain::PresentationLabel,
        "continue",
        "Continue",
        "Continuer",
        None,
    ));
    let locale = LocaleId::new("fr").expect("locale");
    assert_eq!(
        catalog
            .lookup(
                "locked",
                "Requires a key.",
                TextDomain::AvailabilityReason,
                &locale,
                None
            )
            .expect("reason lookup"),
        Some("Nécessite une clé.".to_owned())
    );
    assert_eq!(
        catalog
            .lookup(
                "continue",
                "Continue",
                TextDomain::PresentationLabel,
                &locale,
                None
            )
            .expect("label lookup"),
        Some("Continuer".to_owned())
    );

    let error = catalog.set_plural_forms("fr", "nplurals=2; plural=(n == 42 ? 2 : 0);");
    assert!(error.is_err(), "reachable invalid plural arm was accepted");

    must_ok_unit(catalog.insert("fr", "conflict", "Source.", "Traduction."));
    assert!(
        catalog
            .insert("fr", "conflict", "Source.", "Autre traduction.")
            .is_err(),
        "conflicting locale entries were silently replaced"
    );
}

#[test]
fn catalog_rejects_placeholder_mismatch_and_wrong_plural_arm_count_at_load() {
    let mut catalog = ReciteDialogueCatalog::new();
    must_ok_unit(catalog.set_plural_forms("fr", "nplurals=2; plural=(n != 1);"));
    assert!(
        catalog
            .insert("fr", "line", "Hello {name}.", "Bonjour.")
            .is_err()
    );
    assert!(
        catalog
            .insert_plural(
                "fr",
                "letters",
                "One letter.",
                "{count} letters.",
                vec!["Une lettre.".to_owned()],
                None,
            )
            .is_err()
    );
    assert!(
        catalog
            .insert_plural(
                "fr",
                "letters",
                "One letter.",
                "{count} letters.",
                vec!["Une lettre.".to_owned(), "{other} lettres.".to_owned()],
                None,
            )
            .is_err()
    );
}
