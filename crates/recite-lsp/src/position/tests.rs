use lsp_types::{Position, Range};
use recite_compiler::authoring::{AuthoringKernel, AuthoringRequest, SavedDocument};
use recite_core::{DocumentKey, SourcePosition, SourceSpan};

use super::{document_span_to_range, span_to_range};

#[test]
fn indexed_symbol_ranges_preserve_unicode_and_inclusive_ends()
-> Result<(), Box<dyn std::error::Error>> {
    let source = ":: start\r\n💬café\r\n";
    let mut kernel = AuthoringKernel::new();
    kernel.apply(AuthoringRequest::new(
        kernel.snapshot().generation(),
        [SavedDocument::new(
            DocumentKey::new("source.recite")?,
            source,
        )],
        [],
    ))?;
    let document = &kernel.snapshot().documents()[0];
    let span = SourceSpan::new(
        "source.recite",
        SourcePosition::new(2, 2)?,
        Some(SourcePosition::new(2, 5)?),
    );
    assert_eq!(
        document_span_to_range(document, &span),
        Range::new(Position::new(1, 2), Position::new(1, 6)),
    );
    Ok(())
}

#[test]
fn raw_and_indexed_symbol_ranges_share_line_and_column_clamping()
-> Result<(), Box<dyn std::error::Error>> {
    for source in [
        "",
        "\n",
        "\r",
        ":: start\n",
        ":: start\r\n",
        ":: start\r💬café\rnext",
    ] {
        let mut kernel = AuthoringKernel::new();
        kernel.apply(AuthoringRequest::new(
            kernel.snapshot().generation(),
            [SavedDocument::new(
                DocumentKey::new("source.recite")?,
                source,
            )],
            [],
        ))?;
        let document = &kernel.snapshot().documents()[0];
        for line in [1, 2, 3, 100] {
            for column in [1, 2, 5, 100] {
                let start = SourcePosition::new(line, column)?;
                for end in [None, Some(start), Some(SourcePosition::new(100, 100)?)] {
                    let span = SourceSpan::new("source.recite", start, end);
                    assert_eq!(
                        document_span_to_range(document, &span),
                        span_to_range(source, &span),
                        "source {source:?}, span {span:?}",
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn indexed_cursors_preserve_surrogate_snapping_and_logical_newline_boundaries()
-> Result<(), Box<dyn std::error::Error>> {
    let index = recite_core::SourceLineIndex::new("😀a\rnext\r\nآخر\n");
    for (line, character, expected) in [
        (0, 0, Some((1, 1))),
        (0, 1, Some((1, 1))),
        (0, 2, Some((1, 2))),
        (0, 3, Some((1, 3))),
        (0, 4, None),
        (1, 4, Some((2, 5))),
        (2, 3, Some((3, 4))),
        (3, 0, Some((4, 1))),
        (4, 0, None),
    ] {
        let expected = expected
            .map(|(line, column)| SourcePosition::new(line, column))
            .transpose()?;
        assert_eq!(
            super::indexed_lsp_position_to_source(&index, Position::new(line, character)),
            expected
        );
    }
    Ok(())
}

#[test]
fn indexed_and_text_cursors_agree_on_lf_and_crlf_sources() -> Result<(), Box<dyn std::error::Error>>
{
    for source in ["", "😀a\nآخر\n", "😀a\r\nآخر\r\n"] {
        let index = recite_core::SourceLineIndex::new(source);
        for line in 0..5 {
            for character in 0..8 {
                let position = Position::new(line, character);
                assert_eq!(
                    super::indexed_lsp_position_to_source(&index, position),
                    text_cursor(source, position),
                    "{source:?} at {position:?}"
                );
            }
        }
    }
    Ok(())
}

fn text_cursor(text: &str, position: Position) -> Option<SourcePosition> {
    let line_index = usize::try_from(position.line).ok()?;
    let (line, _) = recite_core::source_lines(text).nth(line_index)?;
    let column = super::scalar_column_for_utf16(line, position.character)?;
    SourcePosition::new(position.line.saturating_add(1), column).ok()
}
