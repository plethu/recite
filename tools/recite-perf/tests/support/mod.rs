use gungraun_summary::either_or_both::EitherOrBoth;
use gungraun_summary::v7::{
    BenchmarkKind, BenchmarkSummary, DhatMetric, EventKind, Metric, MetricResult, MetricResults,
    Profile, ProfileData, ProfileTotal, Profiles, Tool, ToolMetricResults,
};

fn measurement(new: u64, old: u64) -> MetricResult {
    MetricResult {
        change: None,
        values: EitherOrBoth::Both(Metric::Int(new), Metric::Int(old)),
    }
}

fn profile(tool: Tool, metrics: ToolMetricResults) -> Profile {
    Profile {
        data: ProfileData {
            parts: Vec::new(),
            total: ProfileTotal {
                metrics,
                regressions: Vec::new(),
            },
        },
        delay_ns: None,
        duration_ns: 1,
        process_ns: 1,
        setup_ns: None,
        started_at: "2026-10-10T00:00:00Z".into(),
        teardown_ns: None,
        tool,
    }
}

pub(super) fn summary(function: &str, id: &str) -> BenchmarkSummary {
    BenchmarkSummary {
        baselines: (None, Some("control".into())),
        benchmark_exe: "benchmark".into(),
        benchmark_file: "workflows.rs".into(),
        description: None,
        duration_ns: 1,
        function_name: function.into(),
        group: "workflows".into(),
        id: Some(id.into()),
        kind: BenchmarkKind::LibraryBenchmark,
        module_path: format!("workflows::{function}"),
        output_dir: "profiles".into(),
        package_dir: "tools/recite-perf".into(),
        profiles: Profiles(vec![
            profile(
                Tool::Callgrind,
                ToolMetricResults::Callgrind(MetricResults(
                    [(EventKind::Ir, measurement(100, 100))].into(),
                )),
            ),
            profile(
                Tool::DHAT,
                ToolMetricResults::Dhat(MetricResults(
                    [
                        (DhatMetric::TotalBytes, measurement(0, 0)),
                        (DhatMetric::TotalBlocks, measurement(0, 0)),
                        (DhatMetric::AtTGmaxBytes, measurement(0, 0)),
                    ]
                    .into(),
                )),
            ),
        ]),
        project_root: "/benchmark".into(),
        started_at: "2026-10-10T00:00:00Z".into(),
        version: "7".into(),
    }
}

pub(super) fn lines(summaries: &[BenchmarkSummary]) -> String {
    summaries
        .iter()
        .map(|summary| serde_json::to_string(summary).unwrap_or_else(|error| panic!("{error}")))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn writer() -> Vec<BenchmarkSummary> {
    [
        ("preview_start", "passages_1000"),
        ("preview_start", "passages_10000"),
        ("cold_script", "effects_1000"),
        ("cold_script", "effects_10000"),
        ("changed_draft", "wide"),
        ("changed_draft", "dense"),
        ("accepted_draft", "wide"),
        ("accepted_draft", "dense"),
        ("completion_replacement", "wide"),
        ("completion_replacement", "dense"),
    ]
    .into_iter()
    .map(|(function, id)| summary(function, id))
    .collect()
}
