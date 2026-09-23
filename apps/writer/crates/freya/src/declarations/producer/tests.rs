use super::*;

#[test]
fn flatpak_configured_tools_keep_arguments_and_watch_the_host_child() {
    let directory = Path::new("/home/writer/project with spaces");
    let mut command = configured_process(Path::new("./tools/schema producer"), directory, true);
    command.arg("a file; $(not-a-shell).json");
    assert_eq!(command.get_program(), "/usr/bin/flatpak-spawn");
    assert_eq!(command.get_current_dir(), Some(directory));
    assert_eq!(
        command.get_args().collect::<Vec<_>>(),
        [
            "--host",
            "--watch-bus",
            "--directory=/home/writer/project with spaces",
            "--",
            "./tools/schema producer",
            "a file; $(not-a-shell).json",
        ]
    );
}

#[test]
fn native_configured_tools_run_directly() {
    let command = configured_process(Path::new("schema-producer"), Path::new("."), false);
    assert_eq!(command.get_program(), "schema-producer");
    assert_eq!(command.get_args().count(), 0);
}
#[cfg(unix)]
#[test]
fn staged_generation_validates_identity_without_touching_live_output()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let schema = recite_core::SchemaSource::load_str(
        "schema.toml",
        "schema_version=1\n[producer]\nid=\"dialogue\"\n",
    )
    .source
    .ok_or("schema")?
    .export_json();
    std::fs::write(dir.path().join("fixture.json"), &schema)?;
    std::fs::write(dir.path().join("live.json"), "previous output")?;
    let registration = recite_config::ProducerRegistration::parse(
        r#"
version=1
[producer]
kind="standalone"
id="dialogue"
[generate]
program="/bin/sh"
args=["-c", "cp fixture.json \"$1\"", "producer", "{output}"]
"#,
    )?;
    let generated = generate(dir.path(), &registration, &AtomicBool::new(false))?;
    assert_eq!(generated.text, schema);
    assert_eq!(
        std::fs::read_to_string(dir.path().join("live.json"))?,
        "previous output"
    );
    std::fs::write(dir.path().join("fixture.json"), "invalid JSON")?;
    assert!(generate(dir.path(), &registration, &AtomicBool::new(false)).is_err());
    assert_eq!(
        std::fs::read_to_string(dir.path().join("live.json"))?,
        "previous output"
    );
    Ok(())
}
#[cfg(unix)]
#[test]
fn producer_failure_includes_output_and_preserves_arguments()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let registration = recite_config::ProducerRegistration::parse(
        r#"
version=1
[producer]
kind="standalone"
id="dialogue"
[generate]
program="/bin/sh"
args=["-c", "echo 'check adapter types' >&2; exit 7", "producer", "{output}"]
[editor]
program="editor"
args=["--goto", "{file}:{line}:{column}"]
"#,
    )?;
    let error = generate(dir.path(), &registration, &AtomicBool::new(false))
        .err()
        .ok_or("expected failure")?;
    assert!(error.contains("check adapter types"));
    let editor = command(
        dir.path(),
        registration.editor().ok_or("editor")?,
        &[
            ("{file}", "a file.rs".into()),
            ("{line}", "12".into()),
            ("{column}", "3".into()),
        ],
    )?;
    assert_eq!(
        editor.get_args().collect::<Vec<_>>(),
        ["--goto", "a file.rs:12:3"]
    );
    Ok(())
}
