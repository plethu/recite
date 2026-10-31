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
        ["atcf38h", "bt20t3f", "letters.close"]
    );
    assert_eq!(
        dialogue
            .choice_lookup
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        ["ask_news"]
    );

    let arrival = dialogue
        .lines
        .iter()
        .find(|line| line.id.as_str() == "bt20t3f")
        .expect("arrival line");
    assert_eq!(
        arrival.authored_source_text,
        "[slow]The tide is turning, {traveller_name}.[/slow]"
    );
    assert_eq!(arrival.interpolation_bindings[0].name, "traveller_name");

    let plural = dialogue
        .lines
        .iter()
        .find(|line| line.id.as_str() == "atcf38h")
        .expect("plural line");
    assert_eq!(plural.source_text, "You have one letter.");
    assert_eq!(
        plural.plural_source_text.as_deref(),
        Some("You have {count} letters.")
    );
    assert_eq!(plural.interpolation_bindings[0].name, "count");

    let choice = &dialogue.choices[0];
    assert_eq!(choice.id.as_str(), "ask_news");
    assert_eq!(choice.authored_source_text, "Tell me about {topic}.");
    assert_eq!(choice.interpolation_bindings[0].name, "topic");
}
