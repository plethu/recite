use recite_core::{SourceLineIndex, SourcePosition, byte_offset_for_position, source_lines};

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
