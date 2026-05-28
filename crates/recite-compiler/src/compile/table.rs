use super::CompileError;

pub(in crate::compile) fn usize_to_u32(
    table: &'static str,
    value: usize,
) -> Result<u32, CompileError> {
    value
        .try_into()
        .map_err(|_| CompileError::TableIndexOverflow { table, len: value })
}

pub(in crate::compile) fn increment_u32_len(
    table: &'static str,
    len: u32,
) -> Result<u32, CompileError> {
    len.checked_add(1)
        .ok_or_else(|| CompileError::TableIndexOverflow {
            table,
            len: len as usize + 1,
        })
}

#[cfg(test)]
mod tests;
