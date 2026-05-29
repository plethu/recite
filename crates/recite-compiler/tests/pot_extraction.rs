#![cfg(test)]

use std::collections::BTreeMap;

use recite_compiler::{
    CompileInput, PotDocument, PotEntry, PotReference, extract_pot, extract_pot_with_schema,
};
use recite_core::{ProjectSchema, SpeakerDefinition};

#[path = "../../../tests/support/fixtures.rs"]
#[allow(dead_code)]
mod fixture_support;

#[test]
fn extracts_lines_choices_and_speaker_display_names_to_pot() {
    let mut schema = ProjectSchema::empty_v1();
    schema.speakers = BTreeMap::from([
        (
            "narrator".to_owned(),
            SpeakerDefinition {
                display_name: Some("Narrator".to_owned()),
            },
        ),
        (
            "hazel".to_owned(),
            SpeakerDefinition {
                display_name: Some("Hazel".to_owned()),
            },
        ),
        (
            "silent".to_owned(),
            SpeakerDefinition { display_name: None },
        ),
    ]);

    let report = extract_pot_with_schema(project_inputs(), &schema);
    let pot = report.catalog.expect("valid inputs produce a POT catalog");

    fixture_support::assert_text_snapshot(
        &pot.to_pot_string(),
        "pot_extraction__lines_choices_and_speaker_display_names".to_owned(),
    );
}

#[test]
fn structured_entries_preserve_context_before_formatting() {
    let inputs = vec![CompileInput::new(
        "dialogue/prompt.recite",
        concat!(
            ":: start default speaker=narrator\n",
            "> prompt_001 speaker=hazel\n",
            "  Choose.\n",
            "  ? agree_001\n",
            "    Yes.\n",
            "    -> END\n",
        ),
    )];

    let document = extract_pot(inputs).catalog.expect("valid POT catalog");

    assert_eq!(document.entries.len(), 2);
    assert_eq!(document.entries[0].context, "prompt_001");
    assert_eq!(document.entries[0].source_text, "Choose.");
    assert_eq!(
        document.entries[0].comments,
        [
            "file: dialogue/prompt.recite",
            "block: start",
            "speaker: hazel"
        ]
    );
    let first_reference = document.entries[0].reference.as_ref().unwrap();
    assert_eq!(first_reference.line, 3);
    assert_eq!(first_reference.column, 3);

    assert_eq!(document.entries[1].context, "agree_001");
    assert_eq!(document.entries[1].source_text, "Yes.");
    assert_eq!(
        document.entries[1].comments,
        [
            "file: dialogue/prompt.recite",
            "block: start",
            "speaker: hazel"
        ]
    );
    let second_reference = document.entries[1].reference.as_ref().unwrap();
    assert_eq!(second_reference.line, 5);
    assert_eq!(second_reference.column, 5);
}

#[test]
fn extracts_inline_markup_from_recite_source_unchanged() {
    let inputs = vec![CompileInput::new(
        "dialogue/markup.recite",
        concat!(
            ":: start default\n",
            "> marked_line\n",
            "  [slow]Choose[/slow].\n",
            "  ? marked_choice\n",
            "    [shake]Ask now[/shake].\n",
            "    -> END\n",
        ),
    )];

    let document = extract_pot(inputs).catalog.expect("valid POT catalog");

    assert_eq!(document.entries.len(), 2);
    assert_eq!(document.entries[0].context, "marked_line");
    assert_eq!(document.entries[0].source_text, "[slow]Choose[/slow].");
    assert_eq!(document.entries[1].context, "marked_choice");
    assert_eq!(document.entries[1].source_text, "[shake]Ask now[/shake].");
}

#[test]
fn extraction_order_is_independent_of_caller_file_order() {
    let forward = project_inputs();
    let reverse = forward.iter().cloned().rev().collect::<Vec<_>>();

    assert_eq!(
        extract_pot(forward)
            .catalog
            .expect("forward extraction succeeds")
            .to_pot_string(),
        extract_pot(reverse)
            .catalog
            .expect("reverse extraction succeeds")
            .to_pot_string()
    );
}

#[test]
fn validation_failures_return_diagnostics_without_pot() {
    let report = extract_pot([CompileInput::new(
        "dialogue/missing.recite",
        concat!(":: start default\n", ">\n", "  Missing ID.\n",),
    )]);

    assert!(report.catalog.is_none());
    assert_eq!(
        report
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>(),
        ["RECITE_ID001"]
    );
}

#[test]
fn pot_formatter_escapes_gettext_strings() {
    let document = PotDocument {
        entries: vec![PotEntry {
            context: "escape_context".to_owned(),
            source_text: "Quote \" backslash \\ tab\t newline\nplaceholder {name} [slow]x[/slow]"
                .to_owned(),
            comments: Vec::new(),
            reference: Some(PotReference {
                file: "dialogue/escape.recite".to_owned(),
                line: 1,
                column: 1,
            }),
        }],
    };

    fixture_support::assert_text_snapshot(
        &document.to_pot_string(),
        "pot_extraction__formatter_escapes_gettext_strings".to_owned(),
    );
}

#[test]
fn pot_formatter_sanitizes_comments_and_references() {
    let document = PotDocument {
        entries: vec![PotEntry {
            context: "safe".to_owned(),
            source_text: "Text.".to_owned(),
            comments: vec!["file: dialogue\nbad.recite".to_owned()],
            reference: Some(PotReference {
                file: "dialogue/bad\nname:part.recite".to_owned(),
                line: 1,
                column: 1,
            }),
        }],
    };

    fixture_support::assert_text_snapshot(
        &document.to_pot_string(),
        "pot_extraction__formatter_sanitizes_comments_and_references".to_owned(),
    );
}

fn project_inputs() -> Vec<CompileInput> {
    vec![
        CompileInput::new(
            "dialogue/b.recite",
            concat!(
                ":: later\n",
                "> later_line speaker=hazel\n",
                "  Later \"quoted\" text.\n",
            ),
        ),
        CompileInput::new(
            "dialogue/a.recite",
            concat!(
                ":: start default speaker=narrator\n",
                "> intro_001\n",
                "  Welcome\\home.\n",
                "  ? ask_work\n",
                "    Ask about work.\n",
                "    -> dialogue/b.recite::later\n",
                ":if trusts(player)\n",
                "  > secret_001 speaker=hazel\n",
                "    I can tell you.\n",
                ":else\n",
                "  > fallback_001\n",
                "    Not yet.\n",
                ":match stage(thread)\n",
                "  :case ready\n",
                "    > ready_001 speaker=hazel\n",
                "      Ready.\n",
                "  :case _\n",
                "    > waiting_001\n",
                "      Waiting.\n",
            ),
        ),
    ]
}
