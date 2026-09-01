use super::types::SourceRange;
use recite_core::SourcePosition;

pub(super) fn byte_offsets(source: &str, range: SourceRange) -> Result<(usize, usize), ()> {
    if range.start() > range.end() {
        return Err(());
    }
    let start = byte_offset(source, range.start()).ok_or(())?;
    let end = byte_offset(source, range.end()).ok_or(())?;
    (start <= end).then_some((start, end)).ok_or(())
}

fn byte_offset(source: &str, position: SourcePosition) -> Option<usize> {
    let wanted_line = position.line();
    let wanted_scalar = usize::try_from(position.column().checked_sub(1)?).ok()?;
    let bytes = source.as_bytes();
    let mut line_start = 0;
    let mut line = 1;

    for (index, byte) in bytes.iter().copied().enumerate() {
        if byte != b'\n' {
            continue;
        }
        let line_end =
            index.saturating_sub(usize::from(index > line_start && bytes[index - 1] == b'\r'));
        if line == wanted_line {
            return scalar_offset(&source[line_start..line_end], wanted_scalar)
                .map(|offset| line_start + offset);
        }
        line_start = index + 1;
        line = line.saturating_add(1);
    }

    (line == wanted_line)
        .then(|| {
            scalar_offset(&source[line_start..], wanted_scalar).map(|offset| line_start + offset)
        })
        .flatten()
}

fn scalar_offset(line: &str, scalar: usize) -> Option<usize> {
    line.char_indices()
        .nth(scalar)
        .map(|(offset, _)| offset)
        .or_else(|| (line.chars().count() == scalar).then_some(line.len()))
}
