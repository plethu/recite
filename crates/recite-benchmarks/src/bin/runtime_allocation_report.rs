use std::alloc::System;
use std::path::PathBuf;
use std::str::FromStr;

use recite_benchmarks::runtime_allocations::{
    RuntimeAllocationOptions, build_runtime_allocation_report,
};
use recite_benchmarks::scale::parse_fixture_list;
use recite_benchmarks::{BenchmarkError, BenchmarkFixture, BenchmarkResult};
use stats_alloc::{INSTRUMENTED_SYSTEM, StatsAlloc};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse()?;
    let report =
        build_runtime_allocation_report(&RuntimeAllocationOptions::new(args.fixtures), GLOBAL)?;
    let output = match args.format {
        OutputFormat::Json => serde_json::to_string_pretty(&report)?,
        OutputFormat::Markdown => report.to_markdown(),
    };

    if let Some(path) = args.output {
        std::fs::write(path, output)?;
    } else {
        println!("{output}");
    }

    Ok(())
}

#[derive(Debug)]
struct Args {
    fixtures: Vec<BenchmarkFixture>,
    format: OutputFormat,
    output: Option<PathBuf>,
}

impl Args {
    fn parse() -> BenchmarkResult<Self> {
        let mut fixtures = BenchmarkFixture::DEFAULT.to_vec();
        let mut format = OutputFormat::Json;
        let mut output = None;
        let mut args = std::env::args().skip(1);

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--fixtures" => {
                    fixtures = parse_fixture_list(&required_value("--fixtures", args.next())?)?;
                }
                "--format" => {
                    format = OutputFormat::from_str(&required_value("--format", args.next())?)?;
                }
                "--output" => {
                    output = Some(PathBuf::from(required_value("--output", args.next())?));
                }
                "--help" | "-h" => return Err(error(usage())),
                other => return Err(error(format!("unknown argument `{other}`\n\n{}", usage()))),
            }
        }

        Ok(Self {
            fixtures,
            format,
            output,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OutputFormat {
    Json,
    Markdown,
}

impl FromStr for OutputFormat {
    type Err = recite_benchmarks::BenchmarkError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "json" => Ok(Self::Json),
            "markdown" | "md" => Ok(Self::Markdown),
            other => Err(error(format!(
                "unknown --format `{other}`; expected `json` or `markdown`"
            ))),
        }
    }
}

fn required_value(flag: &'static str, value: Option<String>) -> BenchmarkResult<String> {
    value.ok_or_else(|| error(format!("{flag} requires a value")))
}

fn error(message: impl Into<String>) -> BenchmarkError {
    BenchmarkError::Message(message.into())
}

fn usage() -> String {
    "usage: runtime_allocation_report [--fixtures tiny,small,realistic:v1-pack] [--format json|markdown] [--output path]".to_owned()
}
