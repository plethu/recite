//! Immutable accepted inputs for background trial preparation.
use super::{Document, ProjectContext};
use crate::{Preview, PreviewError, PreviewPage, PreviewSetup, Workbench};
use recite_compiler::authoring::AuthoringKernel;
use recite_core::DocumentKey;
use std::sync::Arc;

#[derive(Clone)]
pub struct PreviewRequest {
    source: Arc<str>,
    key: DocumentKey,
    revision: i64,
    pub(crate) kernel: Arc<AuthoringKernel>,
    pub(crate) context: Arc<ProjectContext>,
    block: Option<String>,
}

pub struct PreparedPreview {
    pub(crate) request: PreviewRequest,
    pub(crate) preview: Preview,
    pub(crate) page: PreviewPage,
}

impl PreviewRequest {
    pub(crate) fn capture(document: &Document, block: Option<String>) -> Self {
        Self {
            source: document.source_snapshot(),
            key: document.key.clone(),
            revision: document.version,
            kernel: Arc::clone(&document.kernel),
            context: Arc::clone(&document.context),
            block,
        }
    }
    pub fn is_current(&self, workbench: &Workbench) -> bool {
        let document = workbench.document();
        !workbench.has_draft()
            && self.key == document.key
            && self.revision == document.version
            && Arc::ptr_eq(&self.source, &document.source)
            && Arc::ptr_eq(&self.kernel, &document.kernel)
            && workbench.selected_block().ok() == Some(self.block.clone())
    }
    /// Resolve caller inputs from accepted summaries on the preparing thread.
    pub fn preview_bindings(&self) -> Arc<[recite_core::ast::InterpolationBinding]> {
        crate::projection_cache::preview_bindings::collect(&self.kernel)
    }
    pub fn prepare(&self, setup: PreviewSetup) -> Result<Box<PreparedPreview>, PreviewError> {
        let mut preview = self.compile(setup)?;
        let page = preview.advance(None)?;
        Ok(Box::new(PreparedPreview {
            request: self.clone(),
            preview,
            page,
        }))
    }
    pub(crate) fn compile(&self, setup: PreviewSetup) -> Result<Preview, PreviewError> {
        Preview::from_request(self, self.block.as_deref(), self.revision, setup)
    }
}
