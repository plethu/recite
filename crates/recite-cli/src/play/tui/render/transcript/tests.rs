use super::*;
use std::cell::Cell;

fn entry(text: &str) -> TuiTranscriptEntry {
    TuiTranscriptEntry {
        kind: TuiTranscriptKind::Line,
        id: Some("line_id".into()),
        text: text.into(),
    }
}
fn original_tail<'a>(
    entries: &'a [TuiTranscriptEntry],
    width: u16,
    height: u16,
    messages: &'a Messages,
) -> Text<'a> {
    let mut lines = Vec::new();
    for entry in entries {
        lines.extend(render_transcript_entry(
            entry,
            width as usize,
            messages,
            TuiPalette::default(),
        ));
        lines.push(Line::from(""));
    }
    if !lines.is_empty() {
        lines.pop();
    }
    let start = lines.len().saturating_sub(height as usize);
    Text::from(lines.split_off(start))
}
#[test]
fn viewport_tail_preserves_logical_lines_separators_and_unicode_wrapping() {
    let messages = Messages::load(&crate::i18n::UiLocale::default()).unwrap();
    let entries = [
        entry(""),
        entry("one two three four five six seven eight"),
        entry("駅で待っている。مرحبا بالعالم e\u{301} 👩🏽‍🚀"),
        entry("final line\nsecond paragraph"),
    ];
    for width in [1, 8, 24, 80] {
        for height in [0, 1, 2, 8, 24] {
            assert_eq!(
                render_transcript(
                    entries.iter(),
                    width,
                    height,
                    &messages,
                    TuiPalette::default()
                ),
                original_tail(&entries, width, height, &messages)
            );
        }
    }
}
#[test]
fn fixed_viewport_does_not_visit_retained_history_prefix() {
    let messages = Messages::load(&crate::i18n::UiLocale::default()).unwrap();
    let mut previous = None;
    for history in [1_000, 10_000] {
        let mut entries = (0..history)
            .map(|_| entry("Historical text is outside the viewport."))
            .collect::<Vec<_>>();
        entries.extend((0..32).map(|_| entry("The same visible tail.")));
        let visits = Cell::new(0);
        let rendered = render_transcript(
            entries.iter().inspect(|_| visits.set(visits.get() + 1)),
            80,
            12,
            &messages,
            TuiPalette::default(),
        );
        assert!(
            visits.get() <= 12,
            "viewport work must not depend on retained history: {}",
            visits.get()
        );
        assert_eq!(rendered, original_tail(&entries, 80, 12, &messages));
        let text = format!("{rendered:?}");
        if let Some(previous) = previous {
            assert_eq!(text, previous);
        }
        previous = Some(text);
        visits.set(0);
        let empty = render_transcript(
            entries.iter().inspect(|_| visits.set(visits.get() + 1)),
            80,
            0,
            &messages,
            TuiPalette::default(),
        );
        assert!(empty.lines.is_empty());
        assert_eq!(visits.get(), 0);
    }
}
