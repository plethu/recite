#![cfg(test)]

use tempfile::TempDir;

mod support;
use support::*;

#[test]
fn trace_metrics_are_opt_in_and_run_rejects_metrics_flag() {
    let temp = TempDir::new().expect("tempdir");
    let source = write_recite(
        temp.path(),
        "dialogue.recite",
        concat!(
            ":: start default\n",
            "> intro\n",
            "  Welcome.\n",
            "  ? help if trusts(player)\n",
            "    Help.\n",
            "    -> help\n",
            "  ? leave\n",
            "    Leave.\n",
            "    -> leave\n",
            ":: help\n",
            "! blocking grant_item(map)\n",
            ":if has_bonus(player)\n",
            "  > bonus\n",
            "    Bonus.\n",
            ":else\n",
            "  > helped\n",
            "    Helped.\n",
            "! deferred finish(help)\n",
            "-> END\n",
            ":: leave\n",
            "> left\n",
            "  Left.\n",
            "-> END\n",
        ),
    );
    let asset = compile_project_asset(temp.path(), &source, "dialogue.recitec", None);
    let fixture = write_file(
        temp.path(),
        "fixture.toml",
        r#"[conditions]
"trusts(player)" = true
"has_bonus(player)" = false

[choices]
intro = "help"

[effects]
auto_ack_blocking = true
"#,
    );

    let trace_output = run(recite()
        .arg("trace")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture));
    trace_output.assert_success().assert_stderr("");
    let trace: serde_json::Value =
        serde_json::from_slice(&trace_output.stdout).expect("trace is JSON");
    assert!(trace.get("metrics").is_none());

    let metrics_output = run(recite()
        .arg("trace")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture)
        .arg("--metrics"));
    metrics_output.assert_success().assert_stderr("");
    let trace: serde_json::Value =
        serde_json::from_slice(&metrics_output.stdout).expect("trace is JSON");
    let metrics = &trace["metrics"];
    assert_eq!(metrics["event_count"], 8);
    assert_eq!(metrics["line_count"], 2);
    assert_eq!(metrics["prompt_count"], 1);
    assert_eq!(metrics["choice_count"], 2);
    assert_eq!(metrics["condition_evaluation_count"], 2);
    assert_eq!(metrics["effect_count"]["deferred"], 1);
    assert_eq!(metrics["effect_count"]["immediate"], 0);
    assert_eq!(metrics["effect_count"]["blocking"], 1);
    assert_eq!(metrics["localization_lookup_count"], 0);
    assert!(metrics["elapsed_traversal_time_ns"].is_number());
    assert!(
        metrics["max_serialized_session_size_bytes"]
            .as_u64()
            .expect("serialized session size is numeric")
            > 0
    );

    let run_output = run(recite()
        .arg("run")
        .arg(&asset)
        .arg("--block")
        .arg("start")
        .arg("--fixture")
        .arg(&fixture)
        .arg("--metrics"));
    run_output.assert_failure();
    run_output.assert_stderr_contains("unexpected argument '--metrics'");
}
