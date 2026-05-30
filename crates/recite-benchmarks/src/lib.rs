//! Shared support for Recite Criterion benchmark targets.

pub mod catalog;
pub mod compiler;
pub mod fixture_context;
pub mod project;
pub mod runtime;
pub mod scale;

pub use scale::BenchmarkScale;

#[derive(Debug, thiserror::Error)]
pub enum BenchmarkError {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Fixture(#[from] recite_fixturegen::FixtureError),
    #[error(transparent)]
    Core(#[from] recite_core::CoreValueError),
    #[error(transparent)]
    CompiledValue(#[from] recite_core::CompiledValueError),
    #[error(transparent)]
    Compile(#[from] recite_compiler::CompileError),
    #[error(transparent)]
    Runtime(#[from] recite_runtime::DialogueError),
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
}

pub type BenchmarkResult<T> = Result<T, BenchmarkError>;

pub(crate) fn error(message: impl Into<String>) -> BenchmarkError {
    BenchmarkError::Message(message.into())
}
