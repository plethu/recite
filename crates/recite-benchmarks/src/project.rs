use std::fs;
use std::path::{Path, PathBuf};

use recite_fixturegen::{FixtureConfigSet, FixtureSummary, write_project};

use crate::{BenchmarkResult, BenchmarkScale, error};

#[derive(Clone, Debug)]
pub struct BenchmarkProject {
    scale: BenchmarkScale,
    root: PathBuf,
    summary: FixtureSummary,
}

impl BenchmarkProject {
    pub fn load(scale: BenchmarkScale) -> BenchmarkResult<Self> {
        let workspace = workspace_root();
        let profiles =
            FixtureConfigSet::load_path(&workspace.join("fixtures/synthetic/profiles.toml"))?;
        let profile = profiles.profile(scale.as_str())?;
        let expected = read_summary(&workspace, scale)?;

        let root = if scale == BenchmarkScale::Tiny {
            workspace.join("fixtures/synthetic/tiny")
        } else {
            let output = workspace
                .join("target/recite-benchmarks/generated")
                .join(scale.as_str());
            if output.exists() {
                fs::remove_dir_all(&output)?;
            }
            let generated = write_project(profile, &output)?;
            if generated != expected {
                return Err(error(format!(
                    "generated `{scale}` fixture summary does not match fixtures/synthetic/summaries/{}.json",
                    scale.as_str()
                )));
            }
            output
        };

        verify_summary_files(&root, &expected)?;
        Ok(Self {
            scale,
            root,
            summary: expected,
        })
    }

    #[must_use]
    pub fn scale(&self) -> BenchmarkScale {
        self.scale
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    #[must_use]
    pub fn summary(&self) -> &FixtureSummary {
        &self.summary
    }

    pub fn source_files(&self) -> BenchmarkResult<Vec<ProjectFile>> {
        let source_root = self.root.join("src");
        let mut paths = fs::read_dir(source_root)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()?;
        paths.sort();
        paths
            .into_iter()
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "recite")
            })
            .map(|path| read_project_file(self.root(), path))
            .collect()
    }

    pub fn schema_file(&self) -> BenchmarkResult<ProjectFile> {
        read_project_file(self.root(), self.root.join("schema/synthetic.schema.json"))
    }

    pub fn runtime_fixture_source(&self) -> BenchmarkResult<String> {
        fs::read_to_string(self.root.join("runtime-fixture.toml")).map_err(Into::into)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectFile {
    pub path: String,
    pub source: String,
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn read_summary(workspace: &Path, scale: BenchmarkScale) -> BenchmarkResult<FixtureSummary> {
    let source = fs::read_to_string(
        workspace
            .join("fixtures/synthetic/summaries")
            .join(format!("{}.json", scale.as_str())),
    )?;
    serde_json::from_str(&source).map_err(|json_error| {
        error(format!(
            "failed to read checked `{scale}` fixture summary: {json_error}"
        ))
    })
}

fn verify_summary_files(root: &Path, summary: &FixtureSummary) -> BenchmarkResult<()> {
    for file in &summary.files {
        let path = root.join(&file.path);
        let bytes = fs::read(&path)?;
        let hash = blake3::hash(&bytes).to_hex().to_string();
        if bytes.len() as u64 != file.bytes || hash != file.blake3 {
            return Err(error(format!(
                "fixture file `{}` does not match checked summary",
                path.display()
            )));
        }
    }
    Ok(())
}

fn read_project_file(root: &Path, path: PathBuf) -> BenchmarkResult<ProjectFile> {
    let source = fs::read_to_string(&path)?;
    let relative = path.strip_prefix(root).map_err(|strip_error| {
        error(format!(
            "fixture path is outside project root: {strip_error}"
        ))
    })?;
    Ok(ProjectFile {
        path: relative.to_string_lossy().replace('\\', "/"),
        source,
    })
}
