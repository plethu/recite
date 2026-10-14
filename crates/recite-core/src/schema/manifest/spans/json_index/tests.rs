use super::super::{ManifestSpans, json_string_equals, source_section_range};
use super::*;
use crate::schema::manifest::lower::ManifestSourceFormat;

fn bounds(range: Option<SourceRange>) -> Option<(usize, usize)> {
    range.map(|range| (range.start, range.end))
}

#[test]
fn indexed_occurrences_match_scanning_for_roles_escapes_and_truncated_boundaries() {
    for source in [
        r#"{"conditions":{"gate_a":{"params":[{"name":"gate_a","type":"bo\u006fl"}],"returns":"bool"},"gate_\u0061":{"params":[]}},"other":"gate_a","text":"é😀\"\\"}"#,
        r#"{"good":"é","unfinished":"bad\"#,
        r#"{"good":"é"} } "after""#,
    ] {
        for range in [
            Some(SourceRange {
                start: 0,
                end: source.len(),
            }),
            source_section_range(source, "conditions"),
        ]
        .into_iter()
        .flatten()
        {
            let index = JsonStringIndex::new(source, range);
            for role in [StringRole::Key, StringRole::AnyKey, StringRole::Value] {
                for needle in [
                    "gate_a",
                    "params",
                    "name",
                    "bool",
                    "é",
                    "é😀\"\\",
                    "after",
                    "absent",
                ] {
                    for after in 0..=source.len() + 1 {
                        assert_eq!(
                            bounds(index.find(needle, role, after)),
                            bounds(reference(source, range, after, needle, role)),
                            "{source:?}: {needle} {role:?} after {after}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn role_cursors_exhaust_independently_and_section_reentry_resets_them() {
    let source =
        r#"{"\u0063onditions":{"gate_\u0061":{"value":"bo\u006fl"},"gate_a":{"value":"bool"}}}"#;
    let range = source_section_range(source, "conditions").unwrap();
    let mut spans = ManifestSpans::new_with_format(source, ManifestSourceFormat::Json, None);
    spans.enter_section(source, "conditions");
    let first = reference(source, range, 0, "gate_a", StringRole::Key).unwrap();
    let expected = spans.position(first.start);
    let first_span = spans.next_string_span("schema.json", source, "gate_a", StringRole::Key);
    assert_eq!(first_span.start, expected);
    let second = spans.next_string_span("schema.json", source, "gate_a", StringRole::Key);
    assert!(second.start.column() > first_span.start.column());
    let exhausted = spans.next_string_span("schema.json", source, "gate_a", StringRole::Key);
    assert_eq!(exhausted.start, crate::source_location::point_one());
    assert_eq!(
        spans.next_string_span("schema.json", source, "gate_a", StringRole::AnyKey),
        first_span
    );
    spans.enter_section(source, "conditions");
    assert_eq!(
        spans.next_string_span("schema.json", source, "gate_a", StringRole::Key),
        first_span
    );
    spans.enter_section(source, "missing");
    assert_eq!(
        spans
            .next_string_span("schema.json", source, "bool", StringRole::Value)
            .start,
        spans.position(
            reference(
                source,
                SourceRange {
                    start: 0,
                    end: source.len()
                },
                0,
                "bool",
                StringRole::Value
            )
            .unwrap()
            .start
        )
    );
}

// Previous whole-range scanner, independent of the indexed iterator.
fn reference(
    source: &str,
    range: SourceRange,
    after: usize,
    needle: &str,
    role: StringRole,
) -> Option<SourceRange> {
    let bytes = source.as_bytes();
    let mut depth = 0usize;
    let mut index = range.start;
    while index < range.end {
        match bytes[index] {
            b'"' => {
                let start = index;
                let end = skip_json_string(bytes, start)?;
                if end >= range.end {
                    return None;
                }
                if start >= after
                    && string_role_matches(source, end + 1, role, depth)
                    && json_string_equals(source, start, end, needle)
                {
                    return Some(SourceRange {
                        start,
                        end: end + 1,
                    });
                }
                index = end + 1;
            }
            b'{' | b'[' => {
                depth += 1;
                index += 1;
            }
            b'}' | b']' => {
                depth = depth.checked_sub(1)?;
                index += 1;
            }
            _ => index += 1,
        }
    }
    None
}
