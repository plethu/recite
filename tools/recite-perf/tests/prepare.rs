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
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
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

#[test]
fn control_adapter_preserves_each_revisions_diagnostic_source_owner() {
    for related_sources in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let candidate = directory.path().join("candidate");
        let control = directory.path().join("control");
        let driver = "crates/recite-lsp/src/bench_support/driver.rs";
        let owner = "crates/recite-lsp/src/workspace/ui.rs";
        let current = ".diagnostic_related_sources(&diagnostics.uri, &diagnostics.diagnostics)";
        let historical = ".diagnostic_sources_for_uri(&diagnostics.uri)";
        let production = if related_sources {
            "fn diagnostic_related_sources() {}"
        } else {
            "fn diagnostic_sources_for_uri() {}"
        };
        for root in [&candidate, &control] {
            fs::create_dir_all(root.join(driver).parent().unwrap()).unwrap();
            fs::create_dir_all(root.join(owner).parent().unwrap()).unwrap();
            fs::write(
                root.join("Cargo.toml"),
                "[workspace]\n[workspace.dependencies]\ntoml_edit = '0.24'\n",
            )
            .unwrap();
            fs::write(root.join(driver), current).unwrap();
            fs::write(root.join(owner), production).unwrap();
        }
        for _ in 0..2 {
            let output = Command::new(env!("CARGO_BIN_EXE_recite-perf"))
                .arg("prepare")
                .arg(&candidate)
                .arg(&control)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(fs::read_to_string(candidate.join(driver)).unwrap(), current);
            assert_eq!(fs::read_to_string(control.join(owner)).unwrap(), production);
            assert_eq!(
                fs::read_to_string(control.join(driver)).unwrap(),
                if related_sources { current } else { historical }
            );
        }
        if !related_sources {
            for unknown in ["no diagnostic adapter", &format!("{current} {current}")] {
                fs::write(control.join(driver), unknown).unwrap();
                let status = Command::new(env!("CARGO_BIN_EXE_recite-perf"))
                    .arg("prepare")
                    .arg(&candidate)
                    .arg(&control)
                    .output()
                    .unwrap();
                assert!(!status.status.success());
                assert_eq!(fs::read_to_string(control.join(driver)).unwrap(), unknown);
            }
        }
    }
}
