mod support;
use support::*;

#[test]
fn help_covers_issue_25_commands_and_options() {
    let output = run(recite().arg("--help"));
    output.assert_success().assert_stderr("");
    for command in [
        "validate",
        "compile",
        "extract",
        "check-ids",
        "check-markup",
        "check-metadata",
        "validate-project",
        "check-fresh",
        "run",
        "trace",
        "play",
    ] {
        output.assert_stdout_contains(command);
    }

    let compile = run(recite().arg("compile").arg("--help"));
    compile.assert_success().assert_stderr("");
    compile.assert_stdout_contains("Usage: recite compile [OPTIONS] --output <OUTPUT> <PATHS>...");
    compile.assert_stdout_contains("--schema <SCHEMA>");

    let extract = run(recite().arg("extract").arg("--help"));
    extract.assert_success().assert_stderr("");
    extract.assert_stdout_contains("--output <OUTPUT>");
    extract.assert_stdout_contains("--schema <SCHEMA>");

    let metadata = run(recite().arg("check-metadata").arg("--help"));
    metadata.assert_success().assert_stderr("");
    metadata.assert_stdout_contains("--schema <SCHEMA>");

    let project = run(recite().arg("validate-project").arg("--help"));
    project.assert_success().assert_stderr("");
    project.assert_stdout_contains("recite.project.toml");

    let fresh = run(recite().arg("check-fresh").arg("--help"));
    fresh.assert_success().assert_stderr("");
    fresh.assert_stdout_contains("compiled assets are fresh");

    let run_help = run(recite().arg("run").arg("--help"));
    run_help.assert_success().assert_stderr("");
    run_help.assert_stdout_contains("--block <BLOCK>");
    run_help.assert_stdout_contains("--fixture <FIXTURE>");

    let trace = run(recite().arg("trace").arg("--help"));
    trace.assert_success().assert_stderr("");
    trace.assert_stdout_contains("--block <BLOCK>");
    trace.assert_stdout_contains("--fixture <FIXTURE>");

    let play = run(recite().arg("play").arg("--help"));
    play.assert_success().assert_stderr("");
    play.assert_stdout_contains("--block <BLOCK>");
    play.assert_stdout_contains("--ui <UI>");
    play.assert_stdout_contains("--keymap <KEYMAP>");
}
