use super::{RunChoice, RunLine, RunTranscript};
use crate::i18n::Messages;
use crate::ink::Ink;

fn messages() -> Messages {
    Messages::load(&crate::i18n::UiLocale::default()).expect("default catalog")
}

fn sample() -> RunTranscript {
    let mut run = RunTranscript::default();
    run.effect(
        "effect:a#1".to_owned(),
        "open_console(radio)".to_owned(),
        "blocking".to_owned(),
    );
    run.acknowledge("effect:a#1");
    run.condition("has_key(cipher)".to_owned(), "true".to_owned());
    run.prompt(
        Some(RunLine {
            id: "ffee6p8".to_owned(),
            speaker: Some("operator".to_owned()),
            text: "LACUNA comes through the carrier in winter static, saying the minister was dead before the train arrived.".to_owned(),
        }),
        vec![
            RunChoice {
                id: "wpgvzx4".to_owned(),
                text: "Ask her to wait.".to_owned(),
                available: true,
            },
            RunChoice {
                id: "hear_lacuna".to_owned(),
                text: "Let her speak.".to_owned(),
                available: false,
            },
        ],
    );
    run.select("wpgvzx4");
    run.line(RunLine {
        id: "q1".to_owned(),
        speaker: None,
        text: "The line goes quiet.".to_owned(),
    });
    run.end(vec!["grant_item(oxide_tape)".to_owned()]);
    run
}

fn render(columns: Option<usize>, ink: Ink) -> String {
    let mut out = Vec::new();
    sample()
        .write(&mut out, &messages(), ink, columns)
        .expect("render transcript");
    String::from_utf8(out).expect("utf-8")
}

#[test]
fn piped_transcript_keeps_one_entry_per_line() {
    let expected = [
        "effect     open_console(radio)  blocking, acknowledged",
        "condition  has_key(cipher)  true",
        "OPERATOR   LACUNA comes through the carrier in winter static, saying the minister was \
         dead before the train arrived.  ffee6p8",
        "     >  1  Ask her to wait.  wpgvzx4",
        "        2  Let her speak. (unavailable)  hear_lacuna",
        "           The line goes quiet.  q1",
        "end        end of the conversation",
        "deferred   grant_item(oxide_tape)",
    ];
    assert_eq!(
        render(None, Ink::default()),
        format!("{}\n", expected.join("\n"))
    );
}

#[test]
fn terminal_transcript_wraps_words_under_their_column() {
    let text = render(Some(60), Ink::default());
    for line in text.lines() {
        assert!(
            unicode_width::UnicodeWidthStr::width(line) <= 60,
            "{line:?} exceeds 60 columns"
        );
    }
    assert!(
        text.contains("OPERATOR   LACUNA comes through the carrier"),
        "{text}"
    );
    assert!(
        text.contains("\n           winter static, saying"),
        "{text}"
    );
    assert!(
        text.contains("ffee6p8\n"),
        "the line ID ends its first row: {text}"
    );
}

#[test]
fn colour_marks_speaker_and_pick_only() {
    let text = render(None, Ink::coloured());
    assert!(text.contains("\u{1b}[1;35mOPERATOR\u{1b}[0m"));
    assert!(text.contains("\u{1b}[1;35m     >  1\u{1b}[0m"));
    assert!(!text.contains("\u{1b}[2m"), "no faint text: {text}");
}
