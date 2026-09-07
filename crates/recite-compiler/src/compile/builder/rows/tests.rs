use recite_core::{SourcePosition, SourceSpan};

use super::super::SourcePositionIndex;
use super::slice_source_span;

fn position(line: u32, column: u32) -> SourcePosition {
    SourcePosition::new(line, column)
        .unwrap_or_else(|error| panic!("test position must be valid: {error:?}"))
}

#[test]
fn source_index_preserves_unicode_scalar_columns_and_crlf_boundaries() {
    let source = "α\r\néx\n";
    let index = SourcePositionIndex::new(source);

    assert_eq!(index.byte_offset(1, 1), Some(0));
    assert_eq!(index.byte_offset(1, 2), Some(2));
    assert_eq!(index.byte_offset(1, 3), Some(3));
    assert_eq!(index.byte_offset(1, 4), None);
    assert_eq!(index.byte_offset(2, 1), Some(4));
    assert_eq!(index.byte_offset(2, 2), Some(6));
    assert_eq!(index.byte_offset(2, 3), Some(7));
    assert_eq!(index.byte_offset(3, 1), Some(8));
    assert_eq!(index.byte_offset(4, 1), None);
}

#[test]
fn source_index_slices_the_same_inclusive_end_span_as_source_text() {
    let source = "α\r\néx\n";
    let index = SourcePositionIndex::new(source);
    let span = SourceSpan::new("main.recite", position(2, 1), Some(position(2, 1)));

    assert_eq!(
        slice_source_span(source, &index, &span).as_deref(),
        Some("é")
    );
}
