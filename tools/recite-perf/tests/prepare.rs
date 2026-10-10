use std::fs;
use std::process::Command;

#[test]
fn harness_overlay_preserves_control_dependency_and_workspace_owners() {
    let directory = tempfile::tempdir().unwrap();
    let candidate = directory.path().join("candidate");
    let control = directory.path().join("control");
    fs::create_dir(&candidate).unwrap();
    fs::create_dir(&control).unwrap();
    fs::write(
        candidate.join("Cargo.toml"),
        "[workspace.dependencies]\nrecite-benchmarks = { path = 'crates/recite-benchmarks' }\ntoml_edit = '0.24'\n",
    )
    .unwrap();
    fs::write(
        control.join("Cargo.toml"),
        "[workspace]\nmembers = ['crates/control']\nexclude = ['custom']\n[workspace.dependencies]\ntoml_edit = '=0.23.0'\nserde = '=1.0.0'\n[workspace.package]\nedition = '2021'\n",
    )
    .unwrap();
    let prepare = || {
        let output = Command::new(env!("CARGO_BIN_EXE_recite-perf"))
            .arg("prepare")
            .arg(&candidate)
            .arg(&control)
            .output()
            .unwrap();
        assert!(output.status.success(), "{:?}", output.stderr);
        fs::read_to_string(control.join("Cargo.toml")).unwrap()
    };
    let first = prepare();
    let manifest = first.parse::<toml_edit::DocumentMut>().unwrap();
    assert_eq!(
        manifest["workspace"]["dependencies"]["toml_edit"].as_str(),
        Some("=0.23.0")
    );
    assert_eq!(
        manifest["workspace"]["dependencies"]["serde"].as_str(),
        Some("=1.0.0")
    );
    assert_eq!(
        manifest["workspace"]["package"]["edition"].as_str(),
        Some("2021")
    );
    assert_eq!(
        manifest["workspace"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap())
            .collect::<Vec<_>>(),
        ["crates/control", "tools/recite-perf"]
    );
    assert!(
        manifest["workspace"]["exclude"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("custom"))
    );
    assert_eq!(prepare(), first);
}
