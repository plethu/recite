use super::*;

fn character_delta(before: &str, after: &str) -> Change {
    let start = before
        .chars()
        .zip(after.chars())
        .take_while(|(a, b)| a == b)
        .map(|(c, _)| c.len_utf8())
        .sum();
    let suffix: usize = before[start..]
        .chars()
        .rev()
        .zip(after[start..].chars().rev())
        .take_while(|(a, b)| a == b)
        .map(|(c, _)| c.len_utf8())
        .sum();
    Change {
        start,
        removed: before[start..before.len() - suffix].into(),
        inserted: after[start..after.len() - suffix].into(),
    }
}

fn assert_delta(before: &str, after: &str) {
    let expected = character_delta(before, after);
    let actual = Change::between(before, after);
    assert_eq!(
        (actual.start, &actual.removed, &actual.inserted),
        (expected.start, &expected.removed, &expected.inserted),
        "{before:?} -> {after:?}"
    );
    assert_eq!(actual.apply(before, false), after);
    assert_eq!(actual.apply(after, true), before);
}

#[test]
fn byte_comparison_matches_character_oracle_for_unicode_and_chunk_boundaries() {
    let alphabet = ["a", "é", "ê", "α", "ı", "🙂", "😃", "\u{301}", "\r", "\n"];
    let mut values = vec![String::new()];
    for a in alphabet {
        values.push(a.to_owned());
        for b in alphabet {
            values.push(format!("{a}{b}"));
        }
    }
    for before in &values {
        for after in &values {
            assert_delta(before, after);
        }
    }
    for size in [0, 1, 30, 31, 32, 33, 63, 64, 65, 96] {
        for padding in ["a", "é", "🙂", "e\u{301}", "\r\n"] {
            let prefix = padding.repeat(size);
            let suffix = padding.repeat(97 - size);
            for (before, after) in [
                ("é", "ê"),
                ("α", "ı"),
                ("🙂", "😃"),
                ("", "世界"),
                ("世界", ""),
                ("x", "longer"),
            ] {
                assert_delta(
                    &format!("{prefix}{before}{suffix}"),
                    &format!("{prefix}{after}{suffix}"),
                );
            }
        }
    }
}
