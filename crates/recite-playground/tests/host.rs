use recite_playground::Playground;
use serde_json::Value;

const SOURCE: &str = include_str!("../../../fixtures/recite/valid/landing-junction.recite");

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn output(json: Result<String, String>) -> TestResult<Value> {
    Ok(serde_json::from_str(&json?)?)
}

#[test]
fn real_scene_compiles_and_choice_paths_are_runtime_owned() -> TestResult {
    for (choice, expected) in [
        ("cbd81a23d91fceb42a23", "A contactor closes."),
        ("cbd81a23d91fceb42a24", "The paper opens"),
    ] {
        let mut host = Playground::new();
        assert_eq!(
            output(host.run(SOURCE))?["line"]["id"],
            "cbd81a23d91fceb42a20"
        );
        assert_eq!(output(host.advance())?["line"]["speaker"], "Technician");
        assert_eq!(output(host.advance())?["line"]["speaker"], "Recording");
        let prompt = output(host.advance())?;
        assert_eq!(prompt["kind"], "prompt");
        assert_eq!(prompt["choices"].as_array().expect("choices").len(), 2);
        assert!(
            output(host.select(choice))?["line"]["text"]
                .as_str()
                .expect("line")
                .contains(expected)
        );
        assert_eq!(output(host.advance())?["kind"], "end");
    }
    Ok(())
}

#[test]
fn edits_use_the_compiler_and_invalid_runs_clear_the_previous_session() -> TestResult {
    let mut host = Playground::new();
    assert!(
        output(host.run(&SOURCE.replace("Junction.", "Changed in the browser.")))?["line"]["text"]
            .as_str()
            .expect("line")
            .contains("Changed in the browser.")
    );
    let failed = output(host.run(":: start default\n-> missing\n"))?;
    assert_eq!(failed["kind"], "diagnostics");
    assert!(
        failed["diagnostics"]
            .as_array()
            .expect("diagnostics")
            .iter()
            .any(|diagnostic| diagnostic["span"]["start"]["line"] == 2)
    );
    assert!(host.advance().is_err());
    assert!(host.run(&"x".repeat(65_537)).is_err());
    Ok(())
}

#[test]
fn invalid_choices_and_silent_cycles_fail_without_fake_host_semantics() -> TestResult {
    let mut host = Playground::new();
    output(host.run(SOURCE))?;
    assert!(host.select("cbd81a23d91fceb42a24").is_err());
    assert!(host.run(":: loop default\n-> loop\n").is_err());
    let event = output(host.run(SOURCE))?;
    assert_eq!(event["line"]["id"], "cbd81a23d91fceb42a20");
    Ok(())
}

#[test]
fn diagnostic_records_preserve_crlf_locations_and_unicode_source() -> TestResult {
    let mut host = Playground::new();
    let failed = output(
        host.run(":: start default\r\n> hello@11111111111111111111\r\n  😀\r\n-> missing\r\n"),
    )?;
    let diagnostics = failed["diagnostics"].as_array().expect("diagnostics");
    assert!(
        diagnostics.iter().any(
            |diagnostic| diagnostic["version"] == 1 && diagnostic["span"]["start"]["line"] == 4
        )
    );
    assert_eq!(
        output(host.run(":: start default\n> hello@11111111111111111111\n  😀\n-> END\n"))?["line"]
            ["text"],
        "😀"
    );
    Ok(())
}

#[test]
fn blocking_effects_need_explicit_simulated_completion_and_queries_have_no_fake_host() -> TestResult
{
    let mut host = Playground::new();
    let event = output(host.run(":: start default\n! blocking restore_power(9223372036854775807)\n> done@11111111111111111111\n  Power requested.\n-> END\n"))?;
    assert_eq!(event["kind"], "effect");
    assert_eq!(event["effect"]["function"], "restore_power");
    assert_eq!(event["effect"]["mode"], "blocking");
    assert_eq!(event["effect"]["args"][0]["type"], "integer");
    assert_eq!(event["effect"]["args"][0]["value"], "9223372036854775807");
    assert!(host.advance().is_err());
    assert_eq!(
        output(host.acknowledge())?["line"]["text"],
        "Power requested."
    );
    assert!(
        host.run(":: start default\n:if has_power()\n  -> END\n:else\n  -> END\n")
            .is_err()
    );
    Ok(())
}
