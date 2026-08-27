#![cfg(test)]

use std::fs;
use std::path::Path;
use std::process::Output;

use recite_benchmarks::BenchmarkScale;
use recite_benchmarks::compiler::CompilerProject;
use recite_benchmarks::project::BenchmarkProject;
use recite_benchmarks::runtime::RuntimeProject;
use tempfile::TempDir;

mod support;
use support::*;

#[test]
fn tiny_scale_cli_stress_smoke() -> Result<(), Box<dyn std::error::Error>> {
    run_scale_cli_stress(BenchmarkScale::Tiny)
}

#[test]
#[ignore = "large generated fixture stress check; run explicitly for issue #69"]
fn large_scale_cli_stress() -> Result<(), Box<dyn std::error::Error>> {
    run_scale_cli_stress(BenchmarkScale::Large)
}

#[test]
#[ignore = "epic generated fixture stress check; run explicitly for issue #69"]
fn epic_scale_cli_stress() -> Result<(), Box<dyn std::error::Error>> {
    run_scale_cli_stress(BenchmarkScale::Epic)
}

fn run_scale_cli_stress(scale: BenchmarkScale) -> Result<(), Box<dyn std::error::Error>> {
    let project = BenchmarkProject::load(scale)?;
    let cli_root = CliProjectRoot::new(&project)?;
    let build_dir = cli_root.path().join("build");
    fs::create_dir_all(&build_dir)?;

    run_project_command(cli_root.path(), ["validate", "src"]).assert_success();
    run_project_command(
        cli_root.path(),
        [
            "compile",
            "--schema",
            "schema/synthetic.schema.json",
            "--output",
            "build/synthetic.recitec",
            "src",
        ],
    )
    .assert_success();
    run_project_command(
        cli_root.path(),
        [
            "extract",
            "--schema",
            "schema/synthetic.schema.json",
            "--output",
            "build/synthetic.pot",
            "src",
        ],
    )
    .assert_success();
    run_project_command(cli_root.path(), ["validate-project", "."]).assert_success();
    run_project_command(cli_root.path(), ["check-fresh", "."]).assert_success();
    run_project_command(
        cli_root.path(),
        [
            "run",
            "build/synthetic.recitec",
            "--block",
            "block_00000",
            "--fixture",
            "runtime-fixture.toml",
        ],
    )
    .assert_success();
    let trace_output = run_project_command(
        cli_root.path(),
        [
            "trace",
            "build/synthetic.recitec",
            "--block",
            "block_00000",
            "--fixture",
            "runtime-fixture.toml",
            "--metrics",
        ],
    );
    trace_output.assert_success().assert_stderr("");
    assert_scale_trace(&trace_output.stdout, scale);

    assert_snapshot_restore_continues_deterministically(&project)?;

    Ok(())
}

struct CliProjectRoot<'a> {
    project: &'a BenchmarkProject,
    temp: Option<TempDir>,
}

impl<'a> CliProjectRoot<'a> {
    fn new(project: &'a BenchmarkProject) -> Result<Self, Box<dyn std::error::Error>> {
        if project.scale() != BenchmarkScale::Tiny {
            return Ok(Self {
                project,
                temp: None,
            });
        }

        let temp = TempDir::new()?;
        for file in &project.summary().files {
            let source = project.root().join(&file.path);
            let destination = temp.path().join(&file.path);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(source, destination)?;
        }

        Ok(Self {
            project,
            temp: Some(temp),
        })
    }

    fn path(&self) -> &Path {
        self.temp
            .as_ref()
            .map_or_else(|| self.project.root(), TempDir::path)
    }
}

fn run_project_command<const N: usize>(root: &Path, args: [&str; N]) -> Output {
    let mut command = recite();
    command.current_dir(root).args(args);
    run(&mut command)
}

fn assert_scale_trace(trace_bytes: &[u8], scale: BenchmarkScale) {
    let trace: serde_json::Value = serde_json::from_slice(trace_bytes)
        .unwrap_or_else(|error| panic!("{} trace is JSON: {error}", scale.as_str()));
    assert_eq!(trace["block"], "block_00000");
    assert_eq!(
        trace["dialogue_locale"],
        "en-US",
        "{} trace uses the generated runtime locale",
        scale.as_str()
    );

    let events = trace["events"]
        .as_array()
        .unwrap_or_else(|| panic!("{} trace events are an array", scale.as_str()));
    assert!(
        !events.is_empty(),
        "{} trace produced no events",
        scale.as_str()
    );
    assert!(
        events.iter().any(|event| event["type"] == "line"),
        "{} trace contains line events",
        scale.as_str()
    );
    assert!(
        events.iter().any(|event| event["type"] == "prompt"),
        "{} trace contains prompt events",
        scale.as_str()
    );
    assert!(
        events
            .iter()
            .any(|event| event["type"] == "choice_selected"),
        "{} trace contains choice selection events",
        scale.as_str()
    );
    assert!(
        events.iter().any(|event| event["type"] == "condition"),
        "{} trace contains condition events",
        scale.as_str()
    );
    assert!(
        events.iter().any(|event| event["type"] == "effect"),
        "{} trace contains effect events",
        scale.as_str()
    );
    assert!(
        events.iter().any(|event| event["type"] == "end"),
        "{} trace reaches the end event",
        scale.as_str()
    );

    assert!(
        events.iter().any(|event| {
            event["type"] == "line"
                && event["line"]["id"].as_str().is_some()
                && event["line"]["source_text"].as_str().is_some()
                && event["line"]["text"]
                    .as_str()
                    .is_some_and(|text| text.starts_with("line translation for "))
                && non_empty_array(&event["line"]["metadata"])
        }),
        "{} trace exposes localised lines with metadata",
        scale.as_str()
    );
    assert!(
        events.iter().any(|event| {
            event["type"] == "prompt"
                && non_empty_array(&event["prompt"]["choices"])
                && event["prompt"]["choices"]
                    .as_array()
                    .is_some_and(|choices| {
                        choices.iter().any(|choice| {
                            choice["id"].as_str().is_some()
                                && choice["text"]
                                    .as_str()
                                    .is_some_and(|text| text.starts_with("choice translation for "))
                                && non_empty_array(&choice["metadata"])
                        })
                    })
        }),
        "{} trace exposes prompt choices with localised text and metadata",
        scale.as_str()
    );
    assert!(
        events.iter().any(|event| {
            event["type"] == "condition"
                && event["condition"]["function"] == "relationship"
                && event["condition"]["result"]["enum"] == "active"
        }),
        "{} trace exposes enum condition results",
        scale.as_str()
    );
    assert!(
        events.iter().any(|event| {
            event["type"] == "condition"
                && ["flag", "counter_gte"].iter().any(|function| {
                    event["condition"]["function"]
                        .as_str()
                        .is_some_and(|actual| actual == *function)
                })
                && event["condition"]["result"].as_bool().is_some()
        }),
        "{} trace exposes boolean condition results",
        scale.as_str()
    );
    assert!(
        events.iter().any(|event| {
            event["type"] == "effect"
                && event["effect"]["mode"] == "immediate"
                && event["effect"]["function"] == "play_sfx"
                && non_empty_array(&event["effect"]["args"])
        }),
        "{} trace exposes immediate effects",
        scale.as_str()
    );
    assert!(
        events.iter().any(|event| {
            event["type"] == "effect"
                && event["effect"]["mode"] == "blocking"
                && event["effect"]["function"] == "advance_thread"
                && non_empty_array(&event["effect"]["args"])
        }),
        "{} trace exposes blocking effects",
        scale.as_str()
    );
    assert!(
        non_empty_array(&trace["final_deferred_effects"]),
        "{} trace exposes final deferred effects",
        scale.as_str()
    );

    let metrics = &trace["metrics"];
    assert_positive_metric(metrics, "event_count", scale);
    assert_positive_metric(metrics, "line_count", scale);
    assert_positive_metric(metrics, "prompt_count", scale);
    assert_positive_metric(metrics, "choice_count", scale);
    assert_positive_metric(metrics, "condition_evaluation_count", scale);
    assert_positive_metric(metrics, "localization_lookup_count", scale);
    assert_positive_metric(metrics, "max_serialized_session_size_bytes", scale);
    assert_positive_metric(&metrics["effect_count"], "deferred", scale);
    assert_positive_metric(&metrics["effect_count"], "immediate", scale);
    assert_positive_metric(&metrics["effect_count"], "blocking", scale);
}

fn assert_snapshot_restore_continues_deterministically(
    project: &BenchmarkProject,
) -> Result<(), Box<dyn std::error::Error>> {
    let compiler = CompilerProject::load(project)?;
    let compiled = compiler.compile_with_schema()?;
    let runtime = RuntimeProject::load(project, &compiled)?;
    let driver = runtime.driver();

    let mut original = driver.session_before_blocking_effect()?;
    let original_effect = driver.blocking_effect(&mut original)?;
    let encoded = driver.encode_session(&original)?;
    let mut restored = driver.decode_session(&encoded)?;

    assert_eq!(
        original.pending_effect(),
        restored.pending_effect(),
        "{} restored pending effect matches the original session",
        project.scale().as_str()
    );
    assert_eq!(
        original_effect,
        driver.blocking_effect(&mut restored)?,
        "{} restored pending effect re-emits deterministically",
        project.scale().as_str()
    );
    driver.acknowledge_blocking(&mut original)?;
    driver.acknowledge_blocking(&mut restored)?;
    assert_eq!(
        driver.next_prompt(&mut original)?,
        driver.next_prompt(&mut restored)?,
        "{} restored pending-effect session continues with the same prompt",
        project.scale().as_str()
    );

    Ok(())
}

fn assert_positive_metric(metrics: &serde_json::Value, key: &str, scale: BenchmarkScale) {
    let Some(value) = metrics[key].as_u64() else {
        panic!("{} metric `{key}` is numeric", scale.as_str());
    };
    assert!(
        value > 0,
        "{} metric `{key}` should be positive",
        scale.as_str()
    );
}

fn non_empty_array(value: &serde_json::Value) -> bool {
    value.as_array().is_some_and(|array| !array.is_empty())
}
