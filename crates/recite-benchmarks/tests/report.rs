use recite_benchmarks::report::{
    BenchGroup, BenchReportOptions, BenchTarget, TimingSummary, build_bench_report,
};
use recite_benchmarks::{BenchmarkFixture, BenchmarkScale};

#[test]
fn tiny_report_includes_deterministic_operation_shape() -> Result<(), Box<dyn std::error::Error>> {
    let report = build_bench_report(
        &BenchReportOptions::new(BenchTarget::Fixtures(vec![BenchmarkFixture::Synthetic(
            BenchmarkScale::Tiny,
        )]))
        .with_groups(vec![BenchGroup::Compiler])
        .with_samples(1),
    )?;

    assert_eq!(report.generated_by, "recite bench");
    assert_eq!(report.sample_count, 1);
    assert_eq!(report.selected_groups, [BenchGroup::Compiler]);
    let target = report.targets.first().expect("tiny target report");
    assert_eq!(target.target, "tiny");
    assert_eq!(
        target
            .operations
            .iter()
            .map(|operation| operation.operation.as_str())
            .collect::<Vec<_>>(),
        [
            "parse",
            "lower",
            "validate",
            "validate_with_schema",
            "compile_with_schema",
            "extract_pot_with_schema",
        ]
    );
    assert!(
        target
            .operations
            .iter()
            .all(|operation| operation.summary.samples_ns.len() == 1)
    );
    Ok(())
}

#[test]
fn fixture_count_metadata_makes_scale_shape_concrete() -> Result<(), Box<dyn std::error::Error>> {
    let report = build_bench_report(
        &BenchReportOptions::new(BenchTarget::Fixtures(vec![BenchmarkFixture::Synthetic(
            BenchmarkScale::Tiny,
        )]))
        .with_groups(vec![BenchGroup::Compiler])
        .with_samples(1),
    )?;

    let counts = &report.targets[0].metadata.counts;
    assert_eq!(counts.source_files, 2);
    assert_eq!(counts.blocks, 10);
    assert_eq!(counts.dialogue_lines, 100);
    assert_eq!(counts.choices, 20);
    assert_eq!(counts.generated_words, Some(1080));
    assert!(counts.project_bytes.expect("project bytes") > 0);
    Ok(())
}

#[test]
fn markdown_renders_counts_timings_and_caveats() -> Result<(), Box<dyn std::error::Error>> {
    let report = build_bench_report(
        &BenchReportOptions::new(BenchTarget::Fixtures(vec![BenchmarkFixture::Synthetic(
            BenchmarkScale::Tiny,
        )]))
        .with_groups(vec![BenchGroup::Compiler])
        .with_samples(1),
    )?;

    let markdown = report.to_markdown();
    assert!(markdown.contains("# Recite Benchmark Report"));
    assert!(markdown.contains("| Blocks | 10 |"));
    assert!(markdown.contains("| compiler | parse |"));
    assert!(markdown.contains("Timing deltas are evidence"));
    Ok(())
}

#[test]
fn baseline_comparison_attaches_matching_operation_deltas() -> Result<(), Box<dyn std::error::Error>>
{
    let baseline = build_bench_report(
        &BenchReportOptions::new(BenchTarget::Fixtures(vec![BenchmarkFixture::Synthetic(
            BenchmarkScale::Tiny,
        )]))
        .with_groups(vec![BenchGroup::Compiler])
        .with_samples(1),
    )?;

    let report = build_bench_report(
        &BenchReportOptions::new(BenchTarget::Fixtures(vec![BenchmarkFixture::Synthetic(
            BenchmarkScale::Tiny,
        )]))
        .with_groups(vec![BenchGroup::Compiler])
        .with_samples(1)
        .with_baseline(baseline.clone()),
    )?;

    let operation = &report.targets[0].operations[0];
    let delta = operation.baseline.as_ref().expect("baseline delta");
    assert_eq!(
        delta.baseline_median_ns,
        baseline.targets[0].operations[0].summary.median_ns
    );
    Ok(())
}

#[test]
fn timing_summary_sorts_samples_for_stable_summary() {
    let summary = TimingSummary::from_samples(vec![30, 10, 20]);
    assert_eq!(summary.samples_ns, [10, 20, 30]);
    assert_eq!(summary.min_ns, 10);
    assert_eq!(summary.median_ns, 20);
    assert_eq!(summary.mean_ns, 20);
    assert_eq!(summary.max_ns, 30);
}
