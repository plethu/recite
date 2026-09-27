use super::types::SourceRange;
use recite_core::byte_offset_for_position;

pub(super) fn byte_offsets(source: &str, range: SourceRange) -> Result<(usize, usize), ()> {
    if range.start() > range.end() {
        return Err(());
    }
    let start = byte_offset_for_position(source, range.start()).ok_or(())?;
    let end = byte_offset_for_position(source, range.end()).ok_or(())?;
    (start <= end).then_some((start, end)).ok_or(())
}
