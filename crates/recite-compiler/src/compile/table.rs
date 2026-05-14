use super::CompileError;

pub(in crate::compile) fn usize_to_u32(
    table: &'static str,
    value: usize,
) -> Result<u32, CompileError> {
    value
        .try_into()
        .map_err(|_| CompileError::TableIndexOverflow { table, len: value })
}
