use super::*;

#[test]
fn extracts_shared_language_pressure_fixture_entries_without_losing_context() {
    const FIXTURE: &str = "fixtures/recite/valid/language_pressure.recite";

    let report = extract_pot([CompileInput::new(
        FIXTURE,
        fixture_support::fixture_source(FIXTURE),
    )]);
    assert!(
        report.is_ok(),
        "pressure fixture extracts: {:?}",
        report.diagnostics
    );
    let document = report.catalog.expect("POT for the pressure fixture");

    assert_eq!(
        document
            .entries
            .iter()
            .map(|entry| entry.context.as_str())
            .collect::<Vec<_>>(),
        [
            "55667788990011223344",
            "66778899001122334455",
            "0a1b2c3d4e5f60718293",
            "77889900112233445566"
        ]
    );

    let arrival = &document.entries[0];
    assert_eq!(
        arrival.source_text,
        "[slow]The tide is turning, {traveller_name}.[/slow]"
    );
    assert!(
        arrival
            .comments
            .iter()
            .any(|comment| { comment == "source id: arrivée.interpolée@55667788990011223344" })
    );

    let choice = &document.entries[1];
    assert_eq!(choice.source_text, "Tell me about {topic}.");

    let plural = &document.entries[2];
    assert_eq!(plural.source_text, "You have one letter.");
    assert_eq!(
        plural.plural_source_text.as_deref(),
        Some("You have {count} letters.")
    );
    assert_eq!(
        plural.reference.as_ref().map(|reference| reference.line),
        Some(12)
    );
}
