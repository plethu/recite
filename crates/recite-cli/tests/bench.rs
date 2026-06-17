#![cfg(test)]

mod support;
use support::*;

use tempfile::TempDir;

#[test]
fn bench_help_documents_report_options() {
    let output = run(recite().arg("bench").arg("--help"));

    output.assert_success().assert_stderr("");
    output.assert_stdout_contains("Usage: recite bench [OPTIONS] <TARGET>");
    output.assert_stdout_contains("--scale <SCALE>");
    output.assert_stdout_contains("--group <GROUP>");
    output.assert_stdout_contains("--format <FORMAT>");
    output.assert_stdout_contains("--baseline <BASELINE>");
    output.assert_stdout_contains("--samples <SAMPLES>");
}

#[test]
fn bench_json_outputs_stable_report_shape() {
    let output = run(recite()
        .arg("bench")
        .arg("tiny")
        .arg("--group")
        .arg("compiler")
        .arg("--format")
        .arg("json")
        .arg("--samples")
        .arg("1"));

    output.assert_success().assert_stderr("");
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("benchmark report JSON");
    assert_eq!(report["generated_by"], "recite bench");
    assert_eq!(report["sample_count"], 1);
    assert_eq!(report["selected_groups"], serde_json::json!(["compiler"]));
    assert_eq!(report["targets"][0]["target"], "tiny");
    assert_eq!(report["targets"][0]["metadata"]["counts"]["blocks"], 10);
    assert_eq!(report["targets"][0]["operations"][0]["operation"], "parse");
}

#[test]
fn bench_markdown_outputs_counts_and_caveats() {
    let output = run(recite()
        .arg("bench")
        .arg("tiny")
        .arg("--group")
        .arg("compiler")
        .arg("--samples")
        .arg("1"));

    output.assert_success().assert_stderr("");
    output.assert_stdout_contains("# Recite Benchmark Report");
    output.assert_stdout_contains("| Blocks | 10 |");
    output.assert_stdout_contains("| compiler | parse |");
    output.assert_stdout_contains("Timing deltas are evidence");
}

#[test]
fn bench_group_filtering_limits_operations() {
    let output = run(recite()
        .arg("bench")
        .arg("tiny")
        .arg("--group")
        .arg("runtime")
        .arg("--format")
        .arg("json")
        .arg("--samples")
        .arg("1"));

    output.assert_success().assert_stderr("");
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("benchmark report JSON");
    assert_eq!(report["selected_groups"], serde_json::json!(["runtime"]));
    let operations = report["targets"][0]["operations"].as_array().expect("ops");
    assert!(
        operations
            .iter()
            .all(|operation| operation["group"] == "runtime")
    );
    assert!(
        operations
            .iter()
            .any(|operation| operation["operation"] == "full_traversal")
    );
}

#[test]
fn bench_scale_selection_reports_each_synthetic_fixture() {
    let output = run(recite()
        .arg("bench")
        .arg("synthetic")
        .arg("--scale")
        .arg("tiny")
        .arg("--scale")
        .arg("small")
        .arg("--group")
        .arg("compiler")
        .arg("--format")
        .arg("json")
        .arg("--samples")
        .arg("1"));

    output.assert_success().assert_stderr("");
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("benchmark report JSON");
    assert_eq!(report["targets"][0]["target"], "tiny");
    assert_eq!(report["targets"][1]["target"], "small");
    assert_eq!(report["targets"][1]["metadata"]["counts"]["blocks"], 100);
}

#[test]
fn bench_baseline_comparison_uses_local_json_snapshot() {
    let temp = TempDir::new().expect("tempdir");
    let baseline = temp.path().join("baseline.json");

    run(recite()
        .arg("bench")
        .arg("tiny")
        .arg("--group")
        .arg("compiler")
        .arg("--format")
        .arg("json")
        .arg("--output")
        .arg(&baseline)
        .arg("--samples")
        .arg("1"))
    .assert_success();

    let output = run(recite()
        .arg("bench")
        .arg("tiny")
        .arg("--group")
        .arg("compiler")
        .arg("--format")
        .arg("json")
        .arg("--baseline")
        .arg(&baseline)
        .arg("--samples")
        .arg("1"));

    output.assert_success().assert_stderr("");
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("benchmark report JSON");
    assert!(
        report["targets"][0]["operations"][0]
            .get("baseline")
            .is_some()
    );
}

#[test]
fn bench_project_root_compiler_smoke() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(temp.path(), "dialogue.recite", project_source());
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    let asset_name = asset
        .file_name()
        .and_then(|name| name.to_str())
        .expect("asset file name");
    write_project_manifest(
        temp.path(),
        &format!(
            r#"[project]
content_set = "bench-smoke"
version = "1"

[[scenes]]
id = "start"
asset = "{asset_name}"
block = "start"
"#
        ),
    );

    let output = run(recite()
        .arg("bench")
        .arg(temp.path())
        .arg("--group")
        .arg("compiler")
        .arg("--format")
        .arg("json")
        .arg("--samples")
        .arg("1"));

    output.assert_success().assert_stderr("");
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("benchmark report JSON");
    assert_eq!(report["targets"][0]["kind"], "project_root");
    assert_eq!(
        report["targets"][0]["operations"][0]["operation"],
        "project_manifest_load"
    );
    assert_eq!(report["targets"][0]["metadata"]["counts"]["blocks"], 1);
}

#[test]
fn bench_rejects_invalid_arguments_and_baselines() {
    let invalid_group = run(recite()
        .arg("bench")
        .arg("tiny")
        .arg("--group")
        .arg("unknown"));
    invalid_group.assert_failure();
    invalid_group.assert_stderr_contains("unknown benchmark group `unknown`");

    let temp = TempDir::new().expect("tempdir");
    let baseline = write_file(temp.path(), "baseline.json", "not json");
    let invalid_baseline = run(recite()
        .arg("bench")
        .arg("tiny")
        .arg("--group")
        .arg("compiler")
        .arg("--baseline")
        .arg(&baseline)
        .arg("--samples")
        .arg("1"));
    invalid_baseline.assert_failure();
    invalid_baseline.assert_stderr_contains("failed to read or write benchmark JSON");
}
