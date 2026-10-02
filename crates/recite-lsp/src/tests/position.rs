use lsp_types::{Position, Range};
use recite_core::{SourcePosition, SourceSpan};

pub(super) fn crlf_and_non_bmp_text_use_utf16_ranges() {
    let range = crate::position::span_to_range(
        ":: tavern\r\n💬oops\r\n",
        &SourceSpan::new(
            "file:///workspace/dialogue/utf16.recite",
            source_position(2, 2),
            Some(source_position(2, 5)),
        ),
    );

    assert_eq!(
        range,
        Range {
            start: Position {
                line: 1,
                character: 2
            },
            end: Position {
                line: 1,
                character: 6
            },
        }
    );
}

fn source_position(line: u32, column: u32) -> SourcePosition {
    match SourcePosition::new(line, column) {
        Ok(position) => position,
        Err(error) => panic!("invalid source position {line}:{column}: {error}"),
    }
}

pub(super) fn indexed_edit_ranges_preserve_crlf_and_utf16_boundaries() {
    use recite_compiler::authoring::SourceRange;
    let text = format!("{}💬café\r\n", "# padding\r\n".repeat(1000));
    let source = recite_core::SourceLineIndex::new(text);
    let range = SourceRange::new(source_position(1001, 2), source_position(1001, 6));
    assert_eq!(
        crate::position::source_range_to_lsp(&source, range),
        Some(Range::new(Position::new(1000, 2), Position::new(1000, 6)))
    );
    let invalid = SourceRange::new(source_position(1001, 2), source_position(1001, 7));
    assert_eq!(crate::position::source_range_to_lsp(&source, invalid), None);
}
