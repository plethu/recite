use std::fs;
use std::path::{Path, PathBuf};

use recite_fixturegen::{FixtureConfigSet, FixtureSummary, write_project};
use serde::Deserialize;

use crate::{BenchmarkFixture, BenchmarkResult, BenchmarkScale, error};

#[derive(Clone, Debug)]
pub struct BenchmarkProject {
    fixture: BenchmarkFixture,
    root: PathBuf,
    summary: ProjectSummary,
}

impl BenchmarkProject {
    pub fn load(scale: BenchmarkScale) -> BenchmarkResult<Self> {
        Self::load_fixture(BenchmarkFixture::Synthetic(scale))
    }

    pub fn load_fixture(fixture: BenchmarkFixture) -> BenchmarkResult<Self> {
        match fixture {
            BenchmarkFixture::Synthetic(scale) => Self::load_synthetic(scale),
            BenchmarkFixture::RealisticV1Pack => Self::load_realistic_v1_pack(),
        }
    }

    fn load_synthetic(scale: BenchmarkScale) -> BenchmarkResult<Self> {
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
            fixture: BenchmarkFixture::Synthetic(scale),
            root,
            summary: ProjectSummary::Synthetic(expected),
        })
    }

    fn load_realistic_v1_pack() -> BenchmarkResult<Self> {
        let workspace = workspace_root();
        let root = workspace.join("fixtures/realistic/v1-pack");
        let summary = read_realistic_summary(&workspace, "v1-pack")?;
        verify_realistic_summary_files(&root, &summary)?;
        Ok(Self {
            fixture: BenchmarkFixture::RealisticV1Pack,
            root,
            summary: ProjectSummary::Realistic(summary),
        })
    }

    #[must_use]
    pub fn scale(&self) -> BenchmarkScale {
        match self.fixture {
            BenchmarkFixture::Synthetic(scale) => scale,
            BenchmarkFixture::RealisticV1Pack => {
                panic!("realistic benchmark fixtures do not have a synthetic scale")
            }
        }
    }

    #[must_use]
    pub fn fixture(&self) -> BenchmarkFixture {
        self.fixture
    }

    #[must_use]
    pub fn fixture_label(&self) -> &'static str {
        self.fixture.as_str()
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    #[must_use]
    pub fn summary(&self) -> &FixtureSummary {
        match &self.summary {
            ProjectSummary::Synthetic(summary) => summary,
            ProjectSummary::Realistic(_) => {
                panic!("realistic benchmark fixtures do not use synthetic fixture summaries")
            }
        }
    }

    #[must_use]
    pub fn realistic_summary(&self) -> Option<&RealisticFixtureSummary> {
        match &self.summary {
            ProjectSummary::Synthetic(_) => None,
            ProjectSummary::Realistic(summary) => Some(summary),
        }
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
        let schema_name = match self.fixture {
            BenchmarkFixture::Synthetic(_) => "synthetic.schema.json",
            BenchmarkFixture::RealisticV1Pack => "realistic.schema.json",
        };
        read_project_file(self.root(), self.root.join("schema").join(schema_name))
    }

    pub fn runtime_fixture_source(&self) -> BenchmarkResult<String> {
        fs::read_to_string(self.root.join("runtime-fixture.toml")).map_err(Into::into)
    }
}

#[derive(Clone, Debug)]
enum ProjectSummary {
    Synthetic(FixtureSummary),
    Realistic(RealisticFixtureSummary),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct RealisticFixtureSummary {
    pub name: String,
    pub fixture: String,
    pub counts: RealisticFixtureCounts,
    pub bytes: u64,
    pub files: Vec<RealisticFixtureFile>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct RealisticFixtureCounts {
    pub source_files: u64,
    pub schema_files: u64,
    pub runtime_fixtures: u64,
    pub locale_catalogs: u64,
    pub recite_lines: u64,
    pub dialogue_lines: u64,
    pub choices: u64,
    pub effects: u64,
    pub conditions: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct RealisticFixtureFile {
    pub path: String,
    pub bytes: u64,
    pub blake3: String,
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

fn read_realistic_summary(
    workspace: &Path,
    name: &str,
) -> BenchmarkResult<RealisticFixtureSummary> {
    let source = fs::read_to_string(
        workspace
            .join("fixtures/realistic/summaries")
            .join(format!("{name}.json")),
    )?;
    serde_json::from_str(&source).map_err(|json_error| {
        error(format!(
            "failed to read checked realistic `{name}` fixture summary: {json_error}"
        ))
    })
}

fn verify_realistic_summary_files(
    root: &Path,
    summary: &RealisticFixtureSummary,
) -> BenchmarkResult<()> {
    let mut total_bytes = 0;
    for file in &summary.files {
        let path = root.join(&file.path);
        let bytes = fs::read(&path)?;
        let hash = blake3::hash(&bytes).to_hex().to_string();
        if bytes.len() as u64 != file.bytes || hash != file.blake3 {
            return Err(error(format!(
                "realistic fixture file `{}` does not match checked summary",
                path.display()
            )));
        }
        total_bytes += file.bytes;
    }
    if total_bytes != summary.bytes {
        return Err(error(format!(
            "realistic fixture byte total is {total_bytes}, expected {}",
            summary.bytes
        )));
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
