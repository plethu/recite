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
