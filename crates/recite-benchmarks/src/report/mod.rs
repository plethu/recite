mod counts;
mod lsp;

pub use counts::BenchCounts;
use counts::compiled_condition_sites;
mod timing;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::time::Instant;

use serde::{Deserialize, Deserializer, Serialize};

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
    /// Revision of timing boundaries, independent of product and wire versions.
    #[serde(default)]
    pub measurement_revision: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct FeatureMetadata {
    pub id_storage: String,
}

impl<'de> Deserialize<'de> for FeatureMetadata {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawFeatureMetadata {
            id_storage: Option<String>,
            small_ids: Option<bool>,
        }

        let raw = RawFeatureMetadata::deserialize(deserializer)?;
        let id_storage = raw.id_storage.unwrap_or_else(|| {
            if raw.small_ids.unwrap_or(false) {
                "compact_str"
            } else {
                "string"
            }
            .to_owned()
        });
        Ok(Self { id_storage })
    }
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
    const MEASUREMENT_REVISION: u32 = 1;
    if options
        .baseline
        .as_ref()
        .is_some_and(|baseline| baseline.build.measurement_revision != MEASUREMENT_REVISION)
    {
        return Err(error(
            "benchmark baseline uses different timing boundaries; collect a fresh baseline",
        ));
    }
    if options
        .baseline
        .as_ref()
        .is_some_and(|baseline| baseline.build.profile != build_profile())
    {
        return Err(error("benchmark baseline uses a different build profile"));
    }
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
            measurement_revision: MEASUREMENT_REVISION,
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

pub(crate) fn timed_operation<T>(
    group: BenchGroup,
    operation: &'static str,
    samples: usize,
    mut measure: impl FnMut() -> BenchmarkResult<T>,
) -> BenchmarkResult<BenchOperationReport> {
    timed_operation_with_setup(group, operation, samples, || Ok(()), |_| measure())
}

#[allow(
    clippy::disallowed_methods,
    reason = "benchmark timing is intentionally outside deterministic runtime measurements"
)]
pub(crate) fn timed_operation_with_setup<I, O>(
    group: BenchGroup,
    operation: &'static str,
    samples: usize,
    mut setup: impl FnMut() -> BenchmarkResult<I>,
    mut measure: impl FnMut(&mut I) -> BenchmarkResult<O>,
) -> BenchmarkResult<BenchOperationReport> {
    let mut timings = Vec::with_capacity(samples);
    for _ in 0..samples {
        let mut input = setup()?;
        let started = Instant::now();
        let output = measure(&mut input)?;
        timings.push(started.elapsed());
        // Keep receivers and returned values alive until after the timer stops.
        std::hint::black_box(&output);
    }
    Ok(BenchOperationReport {
        group,
        operation: operation.to_owned(),
        summary: TimingSummary::from_durations(timings),
        baseline: None,
    })
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
