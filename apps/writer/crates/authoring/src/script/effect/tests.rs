use super::*;
use crate::Document;
use crate::ScriptEntry;

fn readings(source: &str) -> Vec<String> {
    let document = Document::new(source).unwrap();
    document.script().unwrap()[0]
        .entries
        .iter()
        .filter_map(|entry| match entry {
            ScriptEntry::Effect(effect) => Some(effect.reading.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn an_effect_reads_as_its_call_and_when_the_game_receives_it() {
    let source = ":: start\n> @a\n  Hello.\n! immediate set_flag(gate_open, true)\n! deferred push_note(\"Left early\")\n! blocking carry(crate, 2)\n! immediate mark_done()\n-> END\n";
    assert_eq!(
        readings(source),
        [
            "Sets flag (gate_open, true) now",
            "Pushes note (“Left early”) when the scene ends",
            "Carries (crate, 2) now, then waits for the game",
            "Marks done now",
        ]
    );
}

#[test]
fn the_verb_takes_the_third_person() {
    for (verb, read) in [
        ("set", "Sets"),
        ("change", "Changes"),
        ("play", "Plays"),
        ("carry", "Carries"),
        ("fix", "Fixes"),
        ("go", "Goes"),
        ("reach", "Reaches"),
    ] {
        // Appending after earlier words reads the verb alone.
        let mut out = String::from("x ");
        third_person(verb, &mut out);
        assert_eq!(&out[2..], read);
    }
}
