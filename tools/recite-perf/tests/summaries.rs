mod support;

use std::io::Cursor;

use gungraun_summary::either_or_both::EitherOrBoth;
use gungraun_summary::v7::{
    DhatMetric, EventKind, Metric, MetricKind, ToolMetricResults, ToolRegression,
};
use recite_perf::{Scope, check_summaries};
use support::{lines, summary, writer};

#[test]
fn accepts_complete_paired_profiles_including_zero_allocation_work() {
    check_summaries(Cursor::new(lines(&writer())), Scope::Writer).unwrap();
}

#[test]
fn bound_text_requires_the_authored_prompt_batch() {
    let case = summary("prompt", "authored_64");
    check_summaries(
        Cursor::new(lines(std::slice::from_ref(&case))),
        Scope::BoundText,
    )
    .unwrap();
    for incomplete in [
        Vec::new(),
        vec![case.clone(), case],
        vec![summary("prompt", "renamed")],
    ] {
        assert!(check_summaries(Cursor::new(lines(&incomplete)), Scope::BoundText).is_err());
    }
}

#[test]
fn rejects_missing_duplicate_and_unexpected_workloads() {
    let cases = writer();
    for incomplete in [
        Vec::new(),
        cases[..1].to_vec(),
        vec![cases[0].clone(), cases[0].clone()],
        vec![cases[0].clone(), summary("cold_script", "unknown")],
    ] {
        assert!(check_summaries(Cursor::new(lines(&incomplete)), Scope::Writer).is_err());
    }
}

#[test]
fn full_scope_requires_medium_for_every_operation() {
    let mut cases = Vec::new();
    for function in [
        "compile",
        "encode",
        "decode",
        "next_line",
        "localised_next",
        "choose",
        "traverse",
        "preview_recovery",
    ] {
        for id in ["small", "realistic"] {
            cases.push(summary(function, id));
        }
    }
    let pr = lines(&cases);
    check_summaries(Cursor::new(&pr), Scope::Pr).unwrap();
    assert!(check_summaries(Cursor::new(&pr), Scope::Full).is_err());
    for function in [
        "compile",
        "encode",
        "decode",
        "next_line",
        "localised_next",
        "choose",
        "traverse",
        "preview_recovery",
    ] {
        cases.push(summary(function, "medium"));
    }
    check_summaries(Cursor::new(lines(&cases)), Scope::Full).unwrap();
}

#[test]
fn rejects_implicit_baseline_creation_schema_drift_and_malformed_json() {
    let mut cases = writer();
    cases[0].baselines.1 = None;
    assert!(check_summaries(Cursor::new(lines(&cases)), Scope::Writer).is_err());
    cases[0].baselines.1 = Some("control".into());
    cases[0].version = "8".into();
    assert!(check_summaries(Cursor::new(lines(&cases)), Scope::Writer).is_err());
    assert!(check_summaries(Cursor::new("{truncated"), Scope::Writer).is_err());
}

#[test]
fn rejects_missing_profiles_unpaired_and_empty_instruction_measurements() {
    let cases = writer();
    for remaining in [0, 1] {
        let mut incomplete = cases.clone();
        incomplete[0].profiles.0 = vec![incomplete[0].profiles.0[remaining].clone()];
        assert!(check_summaries(Cursor::new(lines(&incomplete)), Scope::Writer).is_err());
    }
    for values in [
        EitherOrBoth::Left(Metric::Int(100)),
        EitherOrBoth::Both(Metric::Int(0), Metric::Int(100)),
    ] {
        let mut incomplete = cases.clone();
        let ToolMetricResults::Callgrind(metrics) =
            &mut incomplete[0].profiles.0[0].data.total.metrics
        else {
            panic!("test profile must be Callgrind");
        };
        metrics.0.get_mut(&EventKind::Ir).unwrap().values = values;
        assert!(check_summaries(Cursor::new(lines(&incomplete)), Scope::Writer).is_err());
    }
    let mut duplicate = cases;
    let profile = duplicate[0].profiles.0[0].clone();
    duplicate[0].profiles.0.push(profile);
    assert!(check_summaries(Cursor::new(lines(&duplicate)), Scope::Writer).is_err());
}

#[test]
fn rejects_missing_heap_counters_and_unexpected_tool_profiles() {
    let mut cases = writer();
    let ToolMetricResults::Dhat(metrics) = &mut cases[0].profiles.0[1].data.total.metrics else {
        panic!("test profile must be DHAT");
    };
    metrics.0.shift_remove(&DhatMetric::TotalBytes);
    assert!(check_summaries(Cursor::new(lines(&cases)), Scope::Writer).is_err());
    cases[0].profiles.0[1].data.total.metrics = ToolMetricResults::None;
    assert!(check_summaries(Cursor::new(lines(&cases)), Scope::Writer).is_err());
}

#[test]
fn reported_regressions_fail_the_check_and_leave_the_input_artifact_intact() {
    let mut cases = writer();
    cases[0].profiles.0[0].data.total.regressions = vec![ToolRegression::Hard {
        metric: MetricKind::Callgrind(EventKind::Ir),
        display: None,
        unit: None,
        new: Metric::Int(100),
        diff: Metric::Int(99),
        limit: Metric::Int(1),
    }];
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("candidate.jsonl");
    let input = lines(&cases);
    std::fs::write(&path, &input).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_recite-perf"))
        .args(["check", "writer"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("regression")
    );
    assert_eq!(std::fs::read_to_string(path).unwrap(), input);
}
