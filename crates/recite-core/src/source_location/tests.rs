use super::{SourcePosition, byte_offset_for_position};

fn pos(line: u32, column: u32) -> SourcePosition {
    SourcePosition::new(line, column).expect("positive test position")
}

#[test]
fn source_positions_use_scalar_columns_and_exclusive_line_ends() {
    let source = "é😀\r\nnext\n";
    assert_eq!(byte_offset_for_position(source, pos(1, 1)), Some(0));
    assert_eq!(byte_offset_for_position(source, pos(1, 2)), Some(2));
    assert_eq!(byte_offset_for_position(source, pos(1, 3)), Some(6));
    assert_eq!(byte_offset_for_position(source, pos(1, 4)), None);
    assert_eq!(byte_offset_for_position(source, pos(2, 1)), Some(8));
    assert_eq!(byte_offset_for_position(source, pos(2, 5)), Some(12));
    assert_eq!(byte_offset_for_position(source, pos(3, 1)), Some(13));
    assert_eq!(byte_offset_for_position(source, pos(3, 2)), None);
    assert_eq!(byte_offset_for_position(source, pos(4, 1)), None);
}

#[test]
fn empty_source_has_one_valid_caret_position() {
    assert_eq!(byte_offset_for_position("", pos(1, 1)), Some(0));
    assert_eq!(byte_offset_for_position("", pos(1, 2)), None);
    assert_eq!(byte_offset_for_position("", pos(2, 1)), None);
}
