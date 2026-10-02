use super::types::SourceRange;
use recite_core::{SourceLineIndex, SourcePosition, byte_offset_for_position};

pub(super) fn byte_offsets(source: &str, range: SourceRange) -> Result<(usize, usize), ()> {
    resolve_range(range, |position| byte_offset_for_position(source, position))
}

pub(super) fn indexed_byte_offsets(
    source: &SourceLineIndex,
    range: SourceRange,
) -> Result<(usize, usize), ()> {
    resolve_range(range, |position| source.byte_offset(position))
}

fn resolve_range(
    range: SourceRange,
    offset: impl Fn(SourcePosition) -> Option<usize>,
) -> Result<(usize, usize), ()> {
    if range.start() > range.end() {
        return Err(());
    }
    let start = offset(range.start()).ok_or(())?;
    let end = offset(range.end()).ok_or(())?;
    (start <= end).then_some((start, end)).ok_or(())
}
