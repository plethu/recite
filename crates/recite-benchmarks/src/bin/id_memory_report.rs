use std::hint::black_box;
use std::str::FromStr;
use std::time::{Duration, Instant};

use recite_benchmarks::compiler::{CompilerProject, lower_inputs, parse_inputs};
use recite_benchmarks::fixture_context::RuntimeFixture;
use recite_benchmarks::id_metrics::{
    IdMetricSet, IdStorageReport, active_storage, compiled_id_metrics, id_storage_report,
    runtime_fixture_id_metrics, source_id_metrics,
};
use recite_benchmarks::project::BenchmarkProject;
use recite_benchmarks::runtime::RuntimeProject;
use recite_benchmarks::scale::parse_scale_list;
use recite_benchmarks::{BenchmarkResult, BenchmarkScale};
use serde::Serialize;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse()?;
    let report = build_report(&args)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

#[derive(Debug)]
struct Args {
    variant: String,
    scales: Vec<BenchmarkScale>,
    repeat: usize,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut variant = active_storage().to_owned();
        let mut scales = BenchmarkScale::DEFAULT.to_vec();
        let mut repeat = 3;
        let mut args = std::env::args().skip(1);

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--variant" => {
                    variant = required_value("--variant", args.next())?;
                }
                "--scales" => {
                    scales = parse_scale_list(&required_value("--scales", args.next())?)
                        .map_err(|error| error.to_string())?;
                }
                "--repeat" => {
                    repeat = usize::from_str(&required_value("--repeat", args.next())?)
                        .map_err(|error| format!("--repeat must be a positive integer: {error}"))?;
                    if repeat == 0 {
                        return Err("--repeat must be greater than zero".to_owned());
                    }
                }
                "--help" | "-h" => return Err(usage()),
                other => return Err(format!("unknown argument `{other}`\n\n{}", usage())),
            }
        }

        Ok(Self {
            variant,
            scales,
            repeat,
        })
    }
}

#[derive(Debug, Serialize)]
struct IdMemoryReport {
    variant: String,
    active_storage: &'static str,
    repeat: usize,
    storage: IdStorageReport,
    scales: Vec<ScaleReport>,
}

#[derive(Debug, Serialize)]
struct ScaleReport {
    scale: &'static str,
    fixture_counts: recite_fixturegen::FixtureCounts,
    source_ids: IdMetricSet,
    compiled_ids: IdMetricSet,
    runtime_fixture_ids: IdMetricSet,
    timings: TimingReport,
}

#[derive(Debug, Serialize)]
struct TimingReport {
    parse: TimingSummary,
    lower: TimingSummary,
    compile_with_schema: TimingSummary,
    full_traversal: TimingSummary,
}

#[derive(Debug, Serialize)]
struct TimingSummary {
    samples_ms: Vec<f64>,
    min_ms: f64,
    mean_ms: f64,
}

fn build_report(args: &Args) -> BenchmarkResult<IdMemoryReport> {
    let mut scales = Vec::with_capacity(args.scales.len());
    for scale in &args.scales {
        scales.push(build_scale_report(*scale, args.repeat)?);
    }

    Ok(IdMemoryReport {
        variant: args.variant.clone(),
        active_storage: active_storage(),
        repeat: args.repeat,
        storage: id_storage_report(),
        scales,
    })
}

fn build_scale_report(scale: BenchmarkScale, repeat: usize) -> BenchmarkResult<ScaleReport> {
    let project = BenchmarkProject::load(scale)?;
    let compiler = CompilerProject::load(&project)?;
    let source_ids = source_id_metrics(&compiler.source_files());
    let parse = time_operation(repeat, || {
        let inputs = compiler.compile_inputs();
        parse_inputs(&inputs).map(|parsed| {
            black_box(parsed);
        })
    })?;
    let lower = time_operation(repeat, || {
        let inputs = compiler.compile_inputs();
        lower_inputs(&inputs).map(|sources| {
            black_box(sources);
        })
    })?;
    let compile_with_schema = time_operation(repeat, || {
        compiler.compile_with_schema().map(|compiled| {
            black_box(compiled);
        })
    })?;
    let compiled = compiler.compile_with_schema()?;
    let compiled_ids = compiled_id_metrics(&compiled.asset().dialogue);
    let runtime_fixture = RuntimeFixture::load(&project.runtime_fixture_source()?)?;
    let runtime_fixture_ids = runtime_fixture_id_metrics(&runtime_fixture);
    let runtime = RuntimeProject::load(&project, &compiled)?;
    let full_traversal = time_operation(repeat, || {
        let events = runtime.driver().full_traversal()?;
        black_box(events);
        Ok(())
    })?;

    Ok(ScaleReport {
        scale: scale.as_str(),
        fixture_counts: project.summary().counts.clone(),
        source_ids,
        compiled_ids,
        runtime_fixture_ids,
        timings: TimingReport {
            parse,
            lower,
            compile_with_schema,
            full_traversal,
        },
    })
}

fn time_operation(
    repeat: usize,
    mut operation: impl FnMut() -> BenchmarkResult<()>,
) -> BenchmarkResult<TimingSummary> {
    let mut samples = Vec::with_capacity(repeat);
    for _ in 0..repeat {
        #[allow(
            clippy::disallowed_methods,
            reason = "benchmark report tool intentionally measures elapsed operation time"
        )]
        let started = Instant::now();
        operation()?;
        samples.push(duration_ms(started.elapsed()));
    }
    Ok(TimingSummary::from_samples(samples))
}

impl TimingSummary {
    fn from_samples(samples_ms: Vec<f64>) -> Self {
        let mut min_ms = f64::INFINITY;
        let mut total_ms = 0.0;
        for sample in &samples_ms {
            min_ms = min_ms.min(*sample);
            total_ms += sample;
        }
        let mean_ms = total_ms / samples_ms.len() as f64;

        Self {
            samples_ms,
            min_ms,
            mean_ms,
        }
    }
}

fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn required_value(flag: &'static str, value: Option<String>) -> Result<String, String> {
    value.ok_or_else(|| format!("{flag} requires a value"))
}

fn usage() -> String {
    "usage: id_memory_report [--variant label] [--scales tiny,small] [--repeat n]".to_owned()
}
