use recite_core::{SourceLineIndex, SourcePosition, byte_offset_for_position};

#[test]
fn indexed_positions_match_exact_scalar_boundaries() -> Result<(), Box<dyn std::error::Error>> {
    for source in [
        "",
        "a",
        "a\n",
        "\n\n",
        "a\r\n🦀e\u{301}\r\n",
        "a\r",
        "🦀界\nend",
    ] {
        let index = SourceLineIndex::new(source);
        assert_eq!(index.source(), source);
        for (line, expected) in source.split('\n').enumerate() {
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
