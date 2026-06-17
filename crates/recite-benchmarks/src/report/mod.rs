use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::{BenchmarkFixture, BenchmarkResult, BenchmarkScale, error};

mod fixture;
mod markdown;
mod project_root;

use fixture::build_fixture_reports;
use project_root::build_project_root_reports;

#[derive(Clone, Debug)]
pub struct BenchReportOptions {
    target: BenchTarget,
    groups: Vec<BenchGroup>,
    samples: usize,
    baseline: Option<BenchReport>,
}

impl BenchReportOptions {
    #[must_use]
    pub fn new(target: BenchTarget) -> Self {
        Self {
            target,
            groups: BenchGroup::all().to_vec(),
            samples: 3,
            baseline: None,
        }
    }

    #[must_use]
    pub fn with_groups(mut self, groups: Vec<BenchGroup>) -> Self {
        self.groups = groups;
        self
    }

    #[must_use]
    pub fn with_samples(mut self, samples: usize) -> Self {
        self.samples = samples;
        self
    }

    #[must_use]
    pub fn with_baseline(mut self, baseline: BenchReport) -> Self {
        self.baseline = Some(baseline);
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BenchTarget {
    Fixtures(Vec<BenchmarkFixture>),
    ProjectRoot(PathBuf),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BenchGroup {
    Compiler,
    Runtime,
    Lsp,
}

impl BenchGroup {
    #[must_use]
    pub const fn all() -> &'static [Self; 3] {
        &[Self::Compiler, Self::Runtime, Self::Lsp]
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Compiler => "compiler",
            Self::Runtime => "runtime",
            Self::Lsp => "lsp",
        }
    }
}

impl std::str::FromStr for BenchGroup {
    type Err = crate::BenchmarkError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "compiler" => Ok(Self::Compiler),
            "runtime" => Ok(Self::Runtime),
            "lsp" => Ok(Self::Lsp),
            other => Err(error(format!(
                "unknown benchmark group `{other}`; expected compiler, runtime, or lsp"
            ))),
        }
    }
}

impl std::fmt::Display for BenchGroup {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BenchReport {
    pub generated_by: String,
    pub recite_version: String,
    pub build: BuildMetadata,
    pub sample_count: usize,
    pub selected_groups: Vec<BenchGroup>,
    pub targets: Vec<BenchTargetReport>,
    pub caveats: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BuildMetadata {
    pub profile: String,
    pub features: FeatureMetadata,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FeatureMetadata {
    pub id_storage: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BenchTargetReport {
    pub target: String,
    pub kind: BenchTargetKind,
    pub metadata: TargetMetadata,
    pub operations: Vec<BenchOperationReport>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BenchTargetKind {
    Fixture,
    ProjectRoot,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TargetMetadata {
    pub fixture: Option<String>,
    pub project_root: Option<String>,
    pub counts: BenchCounts,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Default, Eq, PartialEq, Serialize)]
pub struct BenchCounts {
    pub source_files: u64,
    pub schema_files: u64,
    pub runtime_fixtures: u64,
    pub locale_catalogs: u64,
    pub recite_lines: u64,
    pub blocks: u64,
    pub dialogue_lines: u64,
    pub choices: u64,
    pub effects: u64,
    pub conditions: u64,
    pub generated_words: Option<u64>,
    pub project_bytes: Option<u64>,
    pub compiled_asset_bytes: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BenchOperationReport {
    pub group: BenchGroup,
    pub operation: String,
    pub summary: TimingSummary,
    pub baseline: Option<BaselineDelta>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TimingSummary {
    pub samples_ns: Vec<u128>,
    pub min_ns: u128,
    pub median_ns: u128,
    pub mean_ns: u128,
    pub max_ns: u128,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct BaselineDelta {
    pub baseline_median_ns: u128,
    pub delta_ns: i128,
    pub delta_percent: Option<f64>,
}

pub fn build_bench_report(options: &BenchReportOptions) -> BenchmarkResult<BenchReport> {
    let samples = validate_samples(options.samples)?;
    let groups = selected_groups(&options.groups);
    let mut targets = match &options.target {
        BenchTarget::Fixtures(fixtures) => build_fixture_reports(fixtures, &groups, samples)?,
        BenchTarget::ProjectRoot(project_root) => {
            build_project_root_reports(project_root, &groups, samples)?
        }
    };

    if let Some(baseline) = &options.baseline {
        apply_baseline(&mut targets, baseline);
    }

    Ok(BenchReport {
        generated_by: "recite bench".to_owned(),
        recite_version: env!("CARGO_PKG_VERSION").to_owned(),
        build: BuildMetadata {
            profile: build_profile().to_owned(),
            features: FeatureMetadata {
                id_storage: "compact_str".to_owned(),
            },
        },
        sample_count: samples,
        selected_groups: groups,
        targets,
        caveats: vec![
            "Timing deltas are evidence for this named run profile, not absolute performance guarantees.".to_owned(),
            "cargo bench remains the maintainer microbenchmark harness; recite bench is the stable CLI report surface.".to_owned(),
            "Synthetic scale names are fixture IDs, so reports include concrete project-shape counts.".to_owned(),
            "No hard regression threshold is enforced by this command.".to_owned(),
        ],
    })
}

pub(crate) fn timed_operation(
    group: BenchGroup,
    operation: &'static str,
    samples: usize,
    mut measure: impl FnMut() -> BenchmarkResult<()>,
) -> BenchmarkResult<BenchOperationReport> {
    let mut timings = Vec::with_capacity(samples);
    for _ in 0..samples {
        #[allow(
            clippy::disallowed_methods,
            reason = "benchmark report command intentionally measures elapsed operation time"
        )]
        let started = Instant::now();
        measure()?;
        timings.push(started.elapsed());
    }
    Ok(BenchOperationReport {
        group,
        operation: operation.to_owned(),
        summary: TimingSummary::from_durations(timings),
        baseline: None,
    })
}

impl TimingSummary {
    fn from_durations(durations: Vec<Duration>) -> Self {
        let mut samples_ns = durations
            .into_iter()
            .map(|duration| duration.as_nanos())
            .collect::<Vec<_>>();
        samples_ns.sort_unstable();
        Self::from_sorted_samples(samples_ns)
    }

    #[must_use]
    pub fn from_samples(mut samples_ns: Vec<u128>) -> Self {
        samples_ns.sort_unstable();
        Self::from_sorted_samples(samples_ns)
    }

    fn from_sorted_samples(samples_ns: Vec<u128>) -> Self {
        let len = samples_ns.len();
        let min_ns = samples_ns.first().copied().unwrap_or(0);
        let max_ns = samples_ns.last().copied().unwrap_or(0);
        let median_ns = samples_ns.get(len / 2).copied().unwrap_or(0);
        let mean_ns = if len == 0 {
            0
        } else {
            samples_ns.iter().sum::<u128>() / len as u128
        };
        Self {
            samples_ns,
            min_ns,
            median_ns,
            mean_ns,
            max_ns,
        }
    }
}

fn selected_groups(groups: &[BenchGroup]) -> Vec<BenchGroup> {
    let mut seen = BTreeSet::new();
    let mut selected = Vec::new();
    for group in groups {
        if seen.insert(*group) {
            selected.push(*group);
        }
    }
    selected
}

fn validate_samples(samples: usize) -> BenchmarkResult<usize> {
    if samples == 0 {
        return Err(error("benchmark sample count must be at least 1"));
    }
    Ok(samples)
}

fn apply_baseline(targets: &mut [BenchTargetReport], baseline: &BenchReport) {
    let mut baselines = BTreeMap::new();
    for target in &baseline.targets {
        for operation in &target.operations {
            baselines.insert(
                (
                    target.target.clone(),
                    operation.group,
                    operation.operation.clone(),
                ),
                operation.summary.median_ns,
            );
        }
    }
    for target in targets {
        for operation in &mut target.operations {
            let Some(baseline_median_ns) = baselines.get(&(
                target.target.clone(),
                operation.group,
                operation.operation.clone(),
            )) else {
                continue;
            };
            let delta_ns = operation.summary.median_ns as i128 - *baseline_median_ns as i128;
            let delta_percent = (*baseline_median_ns != 0)
                .then_some((delta_ns as f64 / *baseline_median_ns as f64) * 100.0);
            operation.baseline = Some(BaselineDelta {
                baseline_median_ns: *baseline_median_ns,
                delta_ns,
                delta_percent,
            });
        }
    }
}

fn build_profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}

#[must_use]
pub fn default_fixture_target(scales: &[BenchmarkScale]) -> BenchTarget {
    let fixtures = scales
        .iter()
        .copied()
        .map(BenchmarkFixture::Synthetic)
        .collect();
    BenchTarget::Fixtures(fixtures)
}

#[must_use]
pub fn default_scale() -> BenchmarkScale {
    BenchmarkScale::Tiny
}

#[must_use]
pub fn default_groups() -> Vec<BenchGroup> {
    BenchGroup::all().to_vec()
}
