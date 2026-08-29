use super::*;

#[test]
fn shared_language_pressure_fixture_preserves_ids_forms_and_bindings() {
    let asset = compile_fixture("fixtures/recite/valid/language_pressure.recite");
    let dialogue = &asset.dialogue;

    assert_eq!(
        dialogue
            .block_lookup
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        ["letters", "marche.default"]
    );
    assert_eq!(dialogue.default_block.as_u32(), 0);
    assert_eq!(
        dialogue
            .line_lookup
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        [
            "0a1b2c3d4e5f60718293",
            "55667788990011223344",
            "77889900112233445566"
        ]
    );
    assert_eq!(
        dialogue
            .choice_lookup
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        ["66778899001122334455"]
    );

    let arrival = dialogue
        .lines
        .iter()
        .find(|line| line.id.as_str() == "55667788990011223344")
        .expect("arrival line");
    assert_eq!(
        arrival.authored_source_text,
        "[slow]The tide is turning, {traveller_name}.[/slow]"
    );
    assert_eq!(arrival.interpolation_bindings[0].name, "traveller_name");

    let plural = dialogue
        .lines
        .iter()
        .find(|line| line.id.as_str() == "0a1b2c3d4e5f60718293")
        .expect("plural line");
    assert_eq!(plural.source_text, "You have one letter.");
    assert_eq!(
        plural.plural_source_text.as_deref(),
        Some("You have {count} letters.")
    );
    assert_eq!(plural.interpolation_bindings[0].name, "count");

    let choice = &dialogue.choices[0];
    assert_eq!(choice.id.as_str(), "66778899001122334455");
    assert_eq!(choice.authored_source_text, "Tell me about {topic}.");
    assert_eq!(choice.interpolation_bindings[0].name, "topic");
}
