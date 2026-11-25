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

#[test]
fn control_completion_adapter_preserves_current_source_identity_fast_path() {
    for source_accessor in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let candidate = directory.path().join("candidate");
        let control = directory.path().join("control");
        let harness = "tools/recite-perf/benches/support/completion.rs";
        let owner = "apps/writer/crates/authoring/src/completion.rs";
        let model = "apps/writer/crates/authoring/src/lib.rs";
        let current = "workload.completions.source()";
        let source = format!("{current} {current}");
        let production = if source_accessor {
            "pub fn source(&self) {}"
        } else {
            "pub fn replacement(&self) {}"
        };
        for root in [&candidate, &control] {
            fs::create_dir_all(root.join(harness).parent().unwrap()).unwrap();
            fs::create_dir_all(root.join(owner).parent().unwrap()).unwrap();
            fs::write(
                root.join("Cargo.toml"),
                "[workspace.dependencies]\ntoml_edit = '0.24'\n",
            )
            .unwrap();
            fs::write(root.join(harness), &source).unwrap();
            fs::write(root.join(owner), production).unwrap();
            fs::write(root.join(model), "pub use DocumentVersion;").unwrap();
        }
        let expected = if source_accessor {
            source.clone()
        } else {
            source.replace(current, "workload.workload.draft.as_str()")
        };
        for _ in 0..2 {
            let output = Command::new(env!("CARGO_BIN_EXE_recite-perf"))
                .args(["prepare"])
                .arg(&candidate)
                .arg(&control)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(fs::read_to_string(candidate.join(harness)).unwrap(), source);
            assert_eq!(fs::read_to_string(control.join(owner)).unwrap(), production);
            assert_eq!(fs::read_to_string(control.join(harness)).unwrap(), expected);
        }
        if !source_accessor {
            fs::write(control.join(harness), current).unwrap();
            let status = Command::new(env!("CARGO_BIN_EXE_recite-perf"))
                .arg("prepare")
                .arg(&candidate)
                .arg(&control)
                .output()
                .unwrap();
            assert!(!status.status.success());
        }
    }
}

#[test]
fn control_completion_adapter_keeps_integer_revisions_before_typed_versions() {
    for typed_versions in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let candidate = directory.path().join("candidate");
        let control = directory.path().join("control");
        let harness = "tools/recite-perf/benches/support/completion.rs";
        let owner = "apps/writer/crates/authoring/src/completion.rs";
        let model = "apps/writer/crates/authoring/src/lib.rs";
        let source = "use recite_writer_model::{Document, DocumentVersion, ProjectContext};\n\
                      struct Workload { revision: DocumentVersion, }\n";
        let exports = if typed_versions {
            "pub use recite_compiler::authoring::DocumentVersion;"
        } else {
            "pub use document::Document;"
        };
        for root in [&candidate, &control] {
            fs::create_dir_all(root.join(harness).parent().unwrap()).unwrap();
            fs::create_dir_all(root.join(owner).parent().unwrap()).unwrap();
            fs::write(
                root.join("Cargo.toml"),
                "[workspace.dependencies]\ntoml_edit = '0.24'\n",
            )
            .unwrap();
            fs::write(root.join(harness), source).unwrap();
            fs::write(root.join(owner), "pub fn source(&self) {}").unwrap();
            fs::write(root.join(model), exports).unwrap();
        }
        let expected = if typed_versions {
            source.to_owned()
        } else {
            "use recite_writer_model::{Document, ProjectContext};\n\
             struct Workload { revision: i64, }\n"
                .to_owned()
        };
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
            assert_eq!(fs::read_to_string(candidate.join(harness)).unwrap(), source);
            assert_eq!(fs::read_to_string(control.join(harness)).unwrap(), expected);
        }
        if !typed_versions {
            // A partially changed harness must fail rather than guess.
            let unknown = "use recite_writer_model::{Document, DocumentVersion, ProjectContext};\n";
            fs::write(control.join(harness), unknown).unwrap();
            let status = Command::new(env!("CARGO_BIN_EXE_recite-perf"))
                .arg("prepare")
                .arg(&candidate)
                .arg(&control)
                .output()
                .unwrap();
            assert!(!status.status.success());
            assert_eq!(fs::read_to_string(control.join(harness)).unwrap(), unknown);
        }
    }
}

#[test]
fn control_snapshot_adapter_encodes_sessions_without_assets_before_resume() {
    for resumable in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let candidate = directory.path().join("candidate");
        let control = directory.path().join("control");
        let runtime = "crates/recite-benchmarks/src/runtime.rs";
        let errors = "crates/recite-benchmarks/src/preview_hash_dialogue_errors.rs";
        let owner = "crates/recite-runtime/src/resume.rs";
        let encode = "a(encode_session_messagepack(self.asset, &session));\n\
                      b(encode_session_messagepack(self.asset, session));\n";
        let arm = "        recite_runtime::snapshot::DialogueSessionSnapshotConversionError::InvalidEffectRequest {
            id,
            reason,
        } => {
            tag(hasher, 2);
            hash_text(hasher, id);
            hash_text(hasher, reason);
        }
";
        let hash = format!("match error {{\n{arm}}}\n");
        for root in [&candidate, &control] {
            fs::create_dir_all(root.join(runtime).parent().unwrap()).unwrap();
            fs::write(
                root.join("Cargo.toml"),
                "[workspace.dependencies]\ntoml_edit = '0.24'\n",
            )
            .unwrap();
            fs::write(root.join(runtime), encode).unwrap();
            fs::write(root.join(errors), &hash).unwrap();
        }
        if resumable {
            fs::create_dir_all(control.join(owner).parent().unwrap()).unwrap();
            fs::write(control.join(owner), "").unwrap();
        }
        let (expected_runtime, expected_errors) = if resumable {
            (encode.to_owned(), hash.clone())
        } else {
            (
                "a(encode_session_messagepack(&session));\n\
                 b(encode_session_messagepack(session));\n"
                    .to_owned(),
                "match error {\n        // Snapshot effect requests predate this control.\n}\n"
                    .to_owned(),
            )
        };
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
            assert_eq!(fs::read_to_string(candidate.join(runtime)).unwrap(), encode);
            assert_eq!(
                fs::read_to_string(control.join(runtime)).unwrap(),
                expected_runtime
            );
            assert_eq!(
                fs::read_to_string(control.join(errors)).unwrap(),
                expected_errors
            );
        }
    }
}
