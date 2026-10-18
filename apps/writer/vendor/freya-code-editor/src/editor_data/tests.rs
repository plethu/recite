use super::*;
use std::collections::BTreeMap;

pub(super) fn check_widths(data: &mut CodeEditorData) {
    let mut expected = BTreeMap::new();
    for line in data.rope.lines() {
        *expected.entry(line.len_chars()).or_insert(0) += 1;
    }
    assert_eq!(
        data.metrics.line_widths.longest(&data.rope),
        expected.last_key_value().map_or(0, |(&n, _)| n)
    );
    // A fresh index checks the entire histogram, not only the maximum.
    data.metrics.line_widths.assert_matches(&data.rope);
}

#[test]
fn affected_line_widths_match_rebuild_through_unicode_and_line_break_edits() {
    for text in [
        "",
        "a",
        "\n",
        "a\r\nb\n",
        "a\rx\nz",
        "a\r\r\nb",
        "🦊é\nlast",
        "a\u{b}b\u{c}c\u{85}d\u{2028}e\u{2029}f",
    ] {
        let rope = Rope::from_str(text);
        for start in 0..=rope.len_chars() {
            for end in start..=rope.len_chars() {
                for inserted in ["", "x", "🦊", "\n", "\r", "\r\n", "\u{2028}"] {
                    let mut data = CodeEditorData::new(rope.clone(), None);
                    check_widths(&mut data);
                    let start_utf16 = rope.char_to_utf16_cu(start);
                    data.remove(start_utf16..rope.char_to_utf16_cu(end));
                    check_widths(&mut data);
                    data.insert(inserted, start_utf16);
                    check_widths(&mut data);
                }
            }
        }
    }
}

#[test]
fn width_cache_rebuilds_for_direct_rope_changes_and_history() {
    let mut data = CodeEditorData::new(Rope::from_str("short\nlongest line\nlast"), None);
    check_widths(&mut data);
    data.insert("huge replacement", 0);
    check_widths(&mut data);
    data.undo();
    check_widths(&mut data);
    data.redo();
    check_widths(&mut data);
    data.rope = Rope::from_str("new\nsource");
    check_widths(&mut data);
    data.set("another\r\nsource");
    check_widths(&mut data);
}

#[test]
fn ordinary_edit_width_work_is_bounded_independently_of_document_length() {
    for rows in [10, 1_000, 100_000] {
        let mut data =
            CodeEditorData::new(Rope::from_str(&"short\nlonger line\n".repeat(rows)), None);
        check_widths(&mut data);
        data.metrics.line_widths.visited = 0;
        let row = data.rope.len_lines() / 2;
        let pos = data.rope.char_to_utf16_cu(data.rope.line_to_char(row));
        data.insert("x", pos);
        let max = data.metrics.line_widths.longest(&data.rope);
        assert_eq!(max, 12);
        assert!(
            data.metrics.line_widths.visited <= 6,
            "rows={rows}, visits={}",
            data.metrics.line_widths.visited
        );
        check_widths(&mut data);
    }
}
