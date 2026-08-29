use std::ops::Range;

use super::{CompiledAssetDecodeError, malformed};
use crate::compiled::TableRange;

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

pub(super) fn validate_disjoint_ids<'a>(
    field: &'static str,
    left: impl IntoIterator<Item = &'a str>,
    right: impl IntoIterator<Item = &'a str>,
) -> Result<(), CompiledAssetDecodeError> {
    let mut left = left.into_iter().collect::<Vec<_>>();
    let mut right = right.into_iter().collect::<Vec<_>>();
    left.sort_unstable();
    right.sort_unstable();

    let mut left_index = 0;
    let mut right_index = 0;
    while left_index < left.len() && right_index < right.len() {
        match left[left_index].cmp(right[right_index]) {
            std::cmp::Ordering::Less => left_index += 1,
            std::cmp::Ordering::Greater => right_index += 1,
            std::cmp::Ordering::Equal => {
                return Err(malformed(format!(
                    "{field} must be unique, got duplicate `{}`",
                    left[left_index]
                )));
            }
        }
    }
    Ok(())
}

pub(super) fn validate_lookup_entries<'a>(
    table: &'static str,
    row_ids: Vec<&'a str>,
    entries: impl IntoIterator<Item = (&'a str, u32)>,
) -> Result<(), CompiledAssetDecodeError> {
    let entries = entries.into_iter().collect::<Vec<_>>();
    if entries.len() != row_ids.len() {
        return Err(malformed(format!(
            "{table} has {} entries for {} table rows",
            entries.len(),
            row_ids.len()
        )));
    }

    for (id, index) in entries {
        let Some(row_id) = row_ids.get(index as usize) else {
            return Err(malformed(format!(
                "{table} index {index} is out of range for table length {}",
                row_ids.len()
            )));
        };
        if *row_id != id {
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
