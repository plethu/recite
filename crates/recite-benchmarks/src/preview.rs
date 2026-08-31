use recite_core::CompiledDialogue;
use recite_runtime::{PreviewInputs, PreviewOptions, PreviewSession};

use crate::compiler::CompilerProject;
use crate::fixture_context::RuntimeFixture;
use crate::project::BenchmarkProject;
use crate::{BenchmarkFixture, BenchmarkResult};

pub use crate::preview_retention::{PreviewRetentionReport, PreviewSnapshotShape};
pub use crate::preview_shape::PreviewTraceShape;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewTraversalShape {
    pub event_count: usize,
    /// Number of command outputs at which structured preview state was
    /// projected and hashed.
    pub output_count: usize,
    /// Digest of the complete event stream, retained for event-level parity.
    pub event_hash: String,
    /// Digest of each [`recite_runtime::PreviewOutput`] state in command order.
    pub state_hash: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewEvidenceReport {
    pub fixture: &'static str,
    pub traversal: PreviewTraversalShape,
    pub retention: PreviewRetentionReport,
    pub restore: PreviewRestoreParity,
}

/// A compiled, host-neutral project prepared for preview benchmarks.
#[derive(Clone, Debug)]
pub struct PreviewProject {
    pub(crate) fixture: BenchmarkFixture,
    pub(crate) asset: CompiledDialogue,
    pub(crate) runtime_fixture: RuntimeFixture,
    pub(crate) catalog: crate::catalog::CatalogProvider,
}

impl PreviewProject {
    pub fn load(fixture: BenchmarkFixture) -> BenchmarkResult<Self> {
        let project = BenchmarkProject::load_fixture(fixture)?;
        let compiler = CompilerProject::load(&project)?;
        let compiled = compiler.compile_with_schema()?;
        let runtime_fixture = RuntimeFixture::load(&project.runtime_fixture_source()?)?;
        let catalog = crate::catalog::CatalogProvider::load(&project, &runtime_fixture)?;
        Ok(Self {
            fixture,
            asset: compiled.asset().dialogue.clone(),
            runtime_fixture,
            catalog,
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

    pub fn start(&self) -> BenchmarkResult<PreviewSession<'_>> {
        let options = PreviewOptions::new().with_locale(self.runtime_fixture.locale());
        PreviewSession::new(&self.asset, None, options).map_err(Into::into)
    }

    pub fn inputs(&self) -> PreviewInputs<'_> {
        PreviewInputs::new().with_locale_provider(&self.catalog)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewRestoreParity {
    pub events_match: bool,
    pub original_event_count: usize,
    pub restored_event_count: usize,
}
