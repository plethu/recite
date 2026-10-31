use std::io;

use super::CliError;

impl From<io::Error> for CliError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<recite_core::CoreValueError> for CliError {
    fn from(error: recite_core::CoreValueError) -> Self {
        Self::Core(error)
    }
}

impl From<recite_core::compiled::CompiledValueError> for CliError {
    fn from(error: recite_core::compiled::CompiledValueError) -> Self {
        Self::CompiledValue(error)
    }
}

impl From<recite_compiler::compile::CompileError> for CliError {
    fn from(error: recite_compiler::compile::CompileError) -> Self {
        Self::Compile(error)
    }
}

impl From<recite_runtime::DialogueError> for CliError {
    fn from(error: recite_runtime::DialogueError) -> Self {
        Self::Runtime(error)
    }
}

impl From<recite_benchmarks::BenchmarkError> for CliError {
    fn from(error: recite_benchmarks::BenchmarkError) -> Self {
        Self::Benchmark(error)
    }
}

impl From<recite_config::ConfigError> for CliError {
    fn from(source: recite_config::ConfigError) -> Self {
        Self::UserConfig { source }
    }
}
