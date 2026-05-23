#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub(crate) fn recite() -> Command {
    Command::new(env!("CARGO_BIN_EXE_recite"))
}

pub(crate) fn run(command: &mut Command) -> Output {
    command.output().expect("command runs")
}
pub(crate) fn assert_diagnostic_failure(output: &Output) {
    output.assert_failure();
    output.assert_exit_code(1);
    output.assert_stderr_not_contains("error: diagnostics reported");
}

pub(crate) fn write_recite(root: &Path, name: &str, source: &str) -> PathBuf {
    write_file(root, name, source)
}

pub(crate) fn write_project_manifest(root: &Path, source: &str) -> PathBuf {
    write_file(root, "recite.project.toml", source)
}

pub(crate) fn write_file(root: &Path, name: &str, source: &str) -> PathBuf {
    let path = root.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create test parent directory");
    }
    fs::write(&path, source).expect("write test file");
    path
}

pub(crate) fn compile_project_asset(
    root: &Path,
    source: &Path,
    asset_name: &str,
    schema: Option<&Path>,
) -> PathBuf {
    let output = root.join(asset_name);
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).expect("create compiled asset parent directory");
    }
    let mut command = recite();
    command.arg("compile").arg("--output").arg(&output);
    if let Some(schema) = schema {
        command.arg("--schema").arg(schema);
    }
    let source = source.strip_prefix(root).unwrap_or(source);
    command.current_dir(root).arg(source).assert_success();
    output
}

pub(crate) fn corrupt_compiler_compatibility(asset: &Path) {
    let mut bytes = fs::read(asset).expect("compiled asset bytes");
    assert_eq!(bytes[0], 0x9f, "compiled dialogue is a 15-field array");
    assert_eq!(bytes[1], 0x98, "asset header is an 8-field array");
    assert_eq!(bytes[2], 0, "format version starts at v0");
    bytes[3] = 1;
    fs::write(asset, bytes).expect("corrupt compiler compatibility");
}

pub(crate) fn project_source() -> &'static str {
    ":: start default speaker=hazel\n> intro\n  Hello.\n-> END\n"
}

pub(crate) fn schema_manifest<const N: usize>(speakers: [&str; N]) -> String {
    let speakers = speakers
        .into_iter()
        .map(|speaker| format!(r#""{speaker}":{{}}"#))
        .collect::<Vec<_>>()
        .join(",");
    format!(r#"{{"schema_version":1,"speakers":{{{speakers}}}}}"#)
}

pub(crate) trait CommandExt {
    fn assert_success(&mut self) -> Output;
}

impl CommandExt for Command {
    fn assert_success(&mut self) -> Output {
        let output = run(self);
        output.assert_success();
        output
    }
}

pub(crate) trait OutputExt {
    fn assert_success(&self) -> &Self;
    fn assert_failure(&self) -> &Self;
    fn assert_exit_code(&self, expected: i32) -> &Self;
    fn assert_stdout(&self, expected: &str) -> &Self;
    fn assert_stderr(&self, expected: &str) -> &Self;
    fn assert_stdout_contains(&self, expected: &str);
    fn assert_stderr_contains(&self, expected: &str);
    fn assert_stderr_not_contains(&self, unexpected: &str);
}

impl OutputExt for Output {
    fn assert_success(&self) -> &Self {
        assert!(
            self.status.success(),
            "expected success\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
            self.status,
            stdout(self),
            stderr(self)
        );
        self
    }

    fn assert_failure(&self) -> &Self {
        assert!(
            !self.status.success(),
            "expected failure\nstdout:\n{}\nstderr:\n{}",
            stdout(self),
            stderr(self)
        );
        self
    }

    fn assert_exit_code(&self, expected: i32) -> &Self {
        assert_eq!(
            self.status.code(),
            Some(expected),
            "unexpected exit code\nstdout:\n{}\nstderr:\n{}",
            stdout(self),
            stderr(self)
        );
        self
    }

    fn assert_stdout(&self, expected: &str) -> &Self {
        assert_eq!(stdout(self), expected);
        self
    }

    fn assert_stderr(&self, expected: &str) -> &Self {
        assert_eq!(stderr(self), expected);
        self
    }

    fn assert_stdout_contains(&self, expected: &str) {
        assert!(
            stdout(self).contains(expected),
            "stdout did not contain {expected:?}\nstdout:\n{}",
            stdout(self)
        );
    }

    fn assert_stderr_contains(&self, expected: &str) {
        assert!(
            stderr(self).contains(expected),
            "stderr did not contain {expected:?}\nstderr:\n{}",
            stderr(self)
        );
    }

    fn assert_stderr_not_contains(&self, unexpected: &str) {
        assert!(
            !stderr(self).contains(unexpected),
            "stderr unexpectedly contained {unexpected:?}\nstderr:\n{}",
            stderr(self)
        );
    }
}

pub(crate) fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub(crate) fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
