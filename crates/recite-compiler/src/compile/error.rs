use recite_core::CompiledValueError;

/// Non-content failures that prevent asset output even after validation passes.
#[derive(Debug)]
pub enum CompileError {
    CompiledValue(CompiledValueError),
    InvalidValidatedInput(String),
    Serialization(String),
    TableIndexOverflow { table: &'static str, len: usize },
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CompiledValue(error) => error.fmt(formatter),
            Self::InvalidValidatedInput(message) => formatter.write_str(message),
            Self::Serialization(message) => formatter.write_str(message),
            Self::TableIndexOverflow { table, len } => {
                write!(
                    formatter,
                    "{table} table has {len} rows, exceeding u32 indexes"
                )
            }
        }
    }
}

impl std::error::Error for CompileError {}

impl From<CompiledValueError> for CompileError {
    fn from(error: CompiledValueError) -> Self {
        Self::CompiledValue(error)
    }
}
