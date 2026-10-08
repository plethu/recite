use proptest::prelude::*;
use recite_core::{SourceLineIndex, SourcePosition, byte_offset_for_position, source_lines};

proptest! {
    #[test]
    fn generated_scalar_positions_follow_authored_lines(
        contents in prop::collection::vec("[^\\r\\n]{0,24}", 1..16),
        ending in prop_oneof![Just("\n"), Just("\r"), Just("\r\n")],
        final_ending in any::<bool>(),
    ) {
        let mut source = contents.join(ending);
        if final_ending {
            source.push_str(ending);
        }
        let mut logical_lines = contents;
        if final_ending {
            logical_lines.push(String::new());
        }
        let index = SourceLineIndex::new(source.clone());
        let mut start = 0;
        for (line, content) in logical_lines.iter().enumerate() {
            // The oracle comes from the generated content, not another line parser.
            let boundaries = content.char_indices().map(|(offset, _)| offset)
                .chain(std::iter::once(content.len()));
            for (column, offset) in boundaries.enumerate() {
                let position = SourcePosition::new(line as u32 + 1, column as u32 + 1)
                    .map_err(|error| TestCaseError::fail(error.to_string()))?;
                prop_assert_eq!(index.byte_offset(position), Some(start + offset));
                prop_assert_eq!(byte_offset_for_position(&source, position), Some(start + offset));
            }
            let beyond = SourcePosition::new(line as u32 + 1, content.chars().count() as u32 + 2)
                .map_err(|error| TestCaseError::fail(error.to_string()))?;
            prop_assert_eq!(index.byte_offset(beyond), None);
            start += content.len() + ending.len();
        }
        let beyond = SourcePosition::new(logical_lines.len() as u32 + 1, 1)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        prop_assert_eq!(index.byte_offset(beyond), None);
        prop_assert_eq!(source_lines(&source).map(|(text, terminator)| {
            format!("{text}{terminator}")
        }).collect::<String>(), source);
    }
}

#[test]
fn indexed_positions_match_exact_scalar_boundaries() -> Result<(), Box<dyn std::error::Error>> {
    for (source, expected_lines) in [
        ("", vec![""]),
        ("a", vec!["a"]),
        ("a\n", vec!["a", ""]),
        ("\n\n", vec!["", "", ""]),
        ("a\r\n🦀e\u{301}\r\n", vec!["a\r", "🦀e\u{301}\r", ""]),
        ("a\r", vec!["a", ""]),
        ("🦀界\nend", vec!["🦀界", "end"]),
        ("a\r🦀界\r\nend\n", vec!["a", "🦀界\r", "end", ""]),
    ] {
        let index = SourceLineIndex::new(source);
        assert_eq!(index.source(), source);
        for (line, expected) in expected_lines.into_iter().enumerate() {
            assert_eq!(index.line(line), Some(expected));
        }
        for line in 1..8 {
            for column in 1..12 {
                let position = SourcePosition::new(line, column)?;
                assert_eq!(
                    index.byte_offset(position),
                    byte_offset_for_position(source, position),
                    "{source:?} at {line}:{column}"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn source_line_iteration_preserves_every_terminator_and_final_caret_line() {
    let source = "a😀\r\nb\rc\n";
    assert_eq!(
        source_lines(source).collect::<Vec<_>>(),
        [("a😀", "\r\n"), ("b", "\r"), ("c", "\n"), ("", "")]
    );
    assert_eq!(
        source_lines(source)
            .map(|(content, ending)| format!("{content}{ending}"))
            .collect::<String>(),
        source
    );
    assert_eq!(source_lines("").collect::<Vec<_>>(), [("", "")]);
    assert_eq!(
        source_lines("\r\n\r\n").collect::<Vec<_>>(),
        [("", "\r\n"), ("", "\r\n"), ("", "")]
    );
}
