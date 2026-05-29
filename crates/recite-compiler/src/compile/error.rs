use recite_core::CompiledValueError;

/// Non-content failures that prevent asset output even after validation passes.
#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    #[error(transparent)]
    CompiledValue(#[from] CompiledValueError),
    #[error("{0}")]
    InvalidValidatedInput(String),
    #[error("{0}")]
    Serialization(String),
    #[error("{table} table has {len} rows, exceeding u32 indexes")]
    TableIndexOverflow { table: &'static str, len: usize },
}
