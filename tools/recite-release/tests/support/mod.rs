use serde_json::json;
use std::{
    fmt::Display,
    fs,
    path::Path,
    process::{Command, Output},
};
use tempfile::TempDir;

pub(super) struct Candidate {
    directory: TempDir,
    pub(super) commit: String,
}

impl Candidate {
    pub(super) fn new() -> Self {
        let mut candidate = Self {
            directory: require(tempfile::tempdir()),
            commit: String::new(),
        };
        candidate.write(
            "Cargo.toml",
            b"[workspace.package]\nversion='0.2.0-beta.1'\n",
        );
        candidate.write(
            "apps/writer/Cargo.toml",
            b"[workspace.package]\nversion='0.1.0'\n",
        );
        candidate.write(
            "apps/writer/packaging/common.json",
            br#"{"version":"0.1.0"}"#,
        );
        candidate.write(
            "crates/recite-ffi/Cargo.toml",
            b"[package]\nversion='0.7.0'\n",
        );
        candidate.write(
            "dist-workspace.toml",
            b"[dist]\ntargets=['x86_64-unknown-linux-gnu']\n",
        );
        candidate.write(
            ".gitignore",
            b"/artifacts/\n/plan.json\n/receipt.json\n/assets/\n",
        );
        for args in [
            vec!["init", "--initial-branch=main"],
            vec!["config", "user.name", "Release test"],
            vec!["config", "user.email", "test@example.invalid"],
            vec!["config", "commit.gpgsign", "false"],
            vec!["config", "tag.gpgsign", "false"],
            vec!["add", "."],
            vec!["commit", "-m", "Prepare candidate"],
        ] {
            git(candidate.root(), &args);
        }
        candidate.commit = git(candidate.root(), &["rev-parse", "HEAD"]);
        git(
            candidate.root(),
            &["update-ref", "refs/remotes/origin/main", &candidate.commit],
        );
        candidate.artifacts();
        candidate
    }

    pub(super) fn root(&self) -> &Path {
        self.directory.path()
    }

    pub(super) fn artifacts(&self) {
        let mut artifacts = serde_json::Map::new();
        let mut releases = Vec::new();
        for app in ["recite-cli", "recite-lsp"] {
            let name = format!("{app}.tar.xz");
            self.write(&format!("artifacts/cli/{name}"), b"archive");
            artifacts.insert(name.clone(), json!({"name":name, "kind":"executable-zip", "target_triples":["x86_64-unknown-linux-gnu"]}));
            releases
                .push(json!({"app_name":app, "app_version":"0.2.0-beta.1", "artifacts":[name]}));
        }
        for (platform, extension) in [
            ("linux-x86_64", "deb"),
            ("macos-arm64", "dmg"),
            ("macos-x86_64", "dmg"),
            ("windows-x86_64", "exe"),
            ("flatpak-x86_64", "flatpak"),
            ("flatpak-aarch64", "flatpak"),
        ] {
            self.write(
                &format!("artifacts/writer/recite-writer-{platform}-preview/writer.{extension}"),
                b"installer",
            );
        }
        let plan =
            json!({"announcement_tag":"v0.2.0-beta.1", "releases":releases, "artifacts":artifacts});
        self.write("plan.json", &require(serde_json::to_vec(&plan)));
        self.write(
            "artifacts/cli/dist-manifest.json",
            &require(serde_json::to_vec(&plan)),
        );
    }

    pub(super) fn write(&self, path: &str, bytes: &[u8]) {
        let path = self.root().join(path);
        if let Some(parent) = path.parent() {
            require(fs::create_dir_all(parent));
        }
        require(fs::write(path, bytes));
    }

    pub(super) fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_recite-release"));
        command
            .current_dir(self.root())
            .args(args)
            .env_remove("GITHUB_RUN_ID")
            .env_remove("GITHUB_SHA")
            .env_remove("GITHUB_WORKFLOW_REF");
        command
    }

    pub(super) fn run(&self, args: &[&str]) -> Output {
        require(self.command(args).output())
    }

    pub(super) fn seal(&self) -> Output {
        self.run(&[
            "seal",
            "0.2.0-beta.1",
            &self.commit,
            "plan.json",
            "artifacts",
            "receipt.json",
        ])
    }

    pub(super) fn verify(&self) -> Output {
        self.run(&[
            "verify",
            "0.2.0-beta.1",
            &self.commit,
            "receipt.json",
            "artifacts",
        ])
    }
}

fn require<T, E: Display>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("release fixture: {error}"),
    }
}

pub(super) fn git(root: &Path, args: &[&str]) -> String {
    let output = require(Command::new("git").args(args).current_dir(root).output());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    require(String::from_utf8(output.stdout)).trim().to_owned()
}

pub(super) fn success(output: Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

pub(super) fn failure(output: Output, expected: &str) {
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(expected),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
