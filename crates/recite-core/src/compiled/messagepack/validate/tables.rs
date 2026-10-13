use std::ops::Range;

use super::{CompiledAssetDecodeError, malformed};
use crate::compiled::{ChoiceLookupTable, LineLookupTable, TableRange};

pub(super) fn ensure_availability_reason(
    dialogue: &crate::compiled::CompiledDialogue,
    field: &'static str,
    reason_id: &str,
) -> Result<(), CompiledAssetDecodeError> {
    if dialogue
        .availability_reasons
        .iter()
        .any(|reason| reason.id.as_str() == reason_id)
    {
        return Ok(());
    }
    Err(malformed(format!(
        "{field} references unknown availability reason `{reason_id}`"
    )))
}

pub(super) fn ensure_unique_strings<'a>(
    field: &'static str,
    values: impl IntoIterator<Item = &'a str>,
) -> Result<(), CompiledAssetDecodeError> {
    let mut values = values.into_iter().collect::<Vec<_>>();
    values.sort_unstable();
    for window in values.windows(2) {
        if window[0] == window[1] {
            return Err(malformed(format!(
                "{field} `{}` appears more than once",
                window[0]
            )));
        }
    }
    Ok(())
}

pub(super) fn validate_disjoint_lookup_ids(
    lines: &LineLookupTable,
    choices: &ChoiceLookupTable,
) -> Result<(), CompiledAssetDecodeError> {
    // Lookup constructors enforce sorted, unique IDs. The caller has already
    // checked that these lookups cover their rows, so they can be merged directly.
    let mut left = lines.iter().map(|entry| entry.id.as_str()).peekable();
    let mut right = choices.iter().map(|entry| entry.id.as_str()).peekable();

    while let (Some(left_id), Some(right_id)) = (left.peek(), right.peek()) {
        match left_id.cmp(right_id) {
            std::cmp::Ordering::Less => {
                left.next();
            }
            std::cmp::Ordering::Greater => {
                right.next();
            }
            std::cmp::Ordering::Equal => {
                return Err(malformed(format!(
                    "line and choice ids must be unique, got duplicate `{left_id}`"
                )));
            }
        }
    }
    Ok(())
}

pub(super) fn validate_lookup_entries<'a, Row>(
    table: &'static str,
    rows: &'a [Row],
    row_id: impl Fn(&'a Row) -> &'a str,
    entries: impl ExactSizeIterator<Item = (&'a str, u32)>,
) -> Result<(), CompiledAssetDecodeError> {
    if entries.len() != rows.len() {
        return Err(malformed(format!(
            "{table} has {} entries for {} table rows",
            entries.len(),
            rows.len()
        )));
    }

    for (id, index) in entries {
        let Some(row) = rows.get(index as usize) else {
            return Err(malformed(format!(
                "{table} index {index} is out of range for table length {}",
                rows.len()
            )));
        };
        let row_id = row_id(row);
        if row_id != id {
            return Err(malformed(format!(
                "{table} entry `{id}` points to row `{row_id}` at index {index}"
            )));
        }
    }
    Ok(())
}

pub(super) fn ensure_index(
    field: &'static str,
    table_len: usize,
    index: u32,
) -> Result<(), CompiledAssetDecodeError> {
    if (index as usize) < table_len {
        Ok(())
    } else {
        Err(malformed(format!(
            "{field} index {index} is out of range for table length {table_len}"
        )))
    }
}

pub(super) fn ensure_range<I: Copy>(
    field: &'static str,
    table_len: usize,
    range: TableRange<I>,
    index: impl Fn(I) -> u32,
) -> Result<Range<usize>, CompiledAssetDecodeError> {
    let start = index(range.start) as usize;
    let len = range.len as usize;
    let end = start
        .checked_add(len)
        .ok_or_else(|| malformed(format!("{field} range overflows usize")))?;

    if end > table_len {
        return Err(malformed(format!(
            "{field} range {start}..{end} exceeds table length {table_len}"
        )));
    }

    Ok(start..end)
}
