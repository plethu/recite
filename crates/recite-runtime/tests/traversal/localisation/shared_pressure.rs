use super::*;

#[test]
fn shared_language_pressure_fixture_preserves_localised_markup_and_source_fallback() {
    const FIXTURE: &str = "fixtures/recite/valid/language_pressure.recite";
    let asset = compile_asset(
        FIXTURE,
        include_str!("../../../../../fixtures/recite/valid/language_pressure.recite"),
    );
    let provider = RecordingLocaleProvider::default()
        .with(
            "55667788990011223344",
            TextDomain::Line,
            Some("formal"),
            "[slow]La marée tourne, {traveller_name}.[/slow]",
        )
        .with(
            "66778899001122334455",
            TextDomain::Choice,
            Some("formal"),
            "Parle-moi de {topic}.",
        );
    let mut values = recite_runtime::InterpolationValues::new();
    values.insert(
        "traveller_name".to_owned(),
        recite_core::ScalarValue::from("Mara"),
    );
    values.insert(
        "topic".to_owned(),
        recite_core::ScalarValue::from("la marée"),
    );
    values.insert(
        "letters_remaining".to_owned(),
        recite_core::ScalarValue::from(2_i64),
    );
    let resolution = variant_locale_resolution(&provider, "formal").with_values(&values);
    let mut session = start_scene_with_options(
        &asset,
        None,
        DialogueSessionOptions::new().with_locale(locale("en-GB")),
    )
    .expect("starts shared pressure fixture");

    let DialogueEvent::Prompt { line, choices } =
        runtime_next_with(&asset, &mut session, &EmptyDialogueContext, resolution)
            .expect("emits localised prompt")
    else {
        panic!("expected prompt");
    };
    assert_eq!(
        line.expect("prompt line").text,
        "[slow]La marée tourne, Mara.[/slow]"
    );
    assert_eq!(choices[0].text, "Parle-moi de la marée.");

    let DialogueEvent::Line(line) = runtime_choose_with(
        &asset,
        &mut session,
        ChoiceId::new("66778899001122334455").expect("choice ID"),
        &EmptyDialogueContext,
        resolution,
    )
    .expect("chooses the letters block") else {
        panic!("expected plural line");
    };
    assert_eq!(line.id.as_str(), "0a1b2c3d4e5f60718293");
    assert_eq!(line.source_text, "You have {count} letters.");
    assert_eq!(line.text, "You have 2 letters.");
    assert_eq!(line.plural.as_ref().expect("plural trace").count, 2);

    let DialogueEvent::Line(line) =
        runtime_next_with(&asset, &mut session, &EmptyDialogueContext, resolution)
            .expect("emits source fallback line")
    else {
        panic!("expected closing line");
    };
    assert_eq!(line.id.as_str(), "77889900112233445566");
    assert_eq!(line.text, "The ledger closes.");
}
