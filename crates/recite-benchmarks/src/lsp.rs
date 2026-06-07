use std::fs;
use std::path::{Path, PathBuf};

use recite_lsp::bench_support::{
    LspBenchmarkConfig, LspBenchmarkDriver, LspBenchmarkProbes, LspMemoryReport,
};

use crate::project::BenchmarkProject;
use crate::{BenchmarkFixture, BenchmarkResult};

#[derive(Clone, Debug)]
pub struct LspBenchmarkProject {
    fixture: BenchmarkFixture,
    root: PathBuf,
    schema_path: PathBuf,
}

impl LspBenchmarkProject {
    pub fn load(project: &BenchmarkProject) -> BenchmarkResult<Self> {
        let schema_path = project.root().join(match project.fixture() {
            BenchmarkFixture::Synthetic(_) => "schema/synthetic.schema.json",
            BenchmarkFixture::RealisticV1Pack => "schema/realistic.schema.json",
        });
        Ok(Self {
            fixture: project.fixture(),
            root: project.root().to_owned(),
            schema_path,
        })
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
    pub fn driver(&self) -> LspBenchmarkDriver {
        LspBenchmarkDriver::new(&self.config())
    }

    #[must_use]
    pub fn probes(&self) -> LspBenchmarkProbes {
        self.driver().probes()
    }

    #[must_use]
    pub fn memory_report(&self) -> LspMemoryReport {
        self.driver().memory_report()
    }

    pub fn write_memory_report(&self, output_root: &Path) -> BenchmarkResult<PathBuf> {
        let report = self.memory_report();
        let output = output_root.join(format!("{}.md", self.fixture.asset_stem()));
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&output, report.to_markdown())?;
        Ok(output)
    }

    fn config(&self) -> LspBenchmarkConfig {
        LspBenchmarkConfig::new(vec![self.root.join("src")])
            .with_schema_path(self.schema_path.clone())
    }
}
