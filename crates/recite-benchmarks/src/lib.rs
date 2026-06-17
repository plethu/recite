//! Shared support for Recite Criterion benchmark targets.
//!
//! This crate contains reusable loaders, fixture drivers, and scale
//! selection helpers for the workspace benchmark targets. It is documented so
//! maintainers can extend the benchmark suite without duplicating compiler and
//! runtime setup code.
//!
//! Local benchmark runs default to the small local scales. Heavier scales should
//! be selected explicitly with `RECITE_BENCH_SCALES`.
//!
//! # Example
//!
//! ```
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use std::str::FromStr;
//!
//! use recite_benchmarks::BenchmarkScale;
//!
//! let scale = BenchmarkScale::from_str("tiny")?;
//! assert_eq!(scale.as_str(), "tiny");
//! assert!(BenchmarkScale::DEFAULT.contains(&BenchmarkScale::Tiny));
//! # Ok(())
//! # }
//! ```

pub mod catalog;
pub mod compiler;
pub mod fixture_context;
pub mod id_metrics;
pub mod lsp;
pub mod memory_profiles;
pub mod project;
pub mod report;
pub mod runtime;
pub mod runtime_allocations;
pub mod scale;

pub use scale::{BenchmarkFixture, BenchmarkScale};

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
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type BenchmarkResult<T> = Result<T, BenchmarkError>;

pub(crate) fn error(message: impl Into<String>) -> BenchmarkError {
    BenchmarkError::Message(message.into())
}
