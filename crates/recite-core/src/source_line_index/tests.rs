use super::SourceLineIndex;

#[test]
fn indexed_byte_positions_match_scalar_scanning_at_every_offset() {
    for source in ["", "😀éa", "😀a\rnext\r\nآخر\n", "\r\n\n\r", "x\r", "x\r\n"] {
        let index = SourceLineIndex::new(source);
        for offset in 0..source.len() + 3 {
            assert_eq!(
                index.position_for_byte_offset(offset),
                Some(crate::source_location::position_for_byte_offset(
                    source, offset
                )),
                "{source:?} offset {offset}"
            );
        }
    }
}
