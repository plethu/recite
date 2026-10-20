//! Replace prepared project analysis while retaining all live authoring fields.
use super::{Workbench, WorkbenchError};
use crate::{
    CompletionSnapshot, Document, EditError, ProjectContext, projection_cache::CachedProjections,
};
use recite_compiler::authoring::WorkControl;

pub struct ContextRefreshRequest {
    base: CompletionSnapshot,
    projections: CachedProjections,
}
pub struct PreparedContextRefresh {
    base: CompletionSnapshot,
    document: Document,
    accepted: bool,
}
impl Workbench {
    pub fn context_refresh_request(&self) -> ContextRefreshRequest {
        ContextRefreshRequest {
            base: self.document.completion_snapshot(),
            projections: self.document.projections.capture_local(),
        }
    }
    /// New field typing and selection are independent of accepted analysis freshness.
    pub fn accept_context_refresh(
        &mut self,
        prepared: &mut PreparedContextRefresh,
    ) -> Result<(), WorkbenchError> {
        if prepared.accepted {
            return Err(EditError::Stale.into());
        }
        self.document
            .accept_navigation_analysis(&prepared.base, &mut prepared.document)?;
        self.draft_revision = self.document.revision();
        prepared.accepted = true;
        Ok(())
    }
}
impl ContextRefreshRequest {
    pub fn prepare(
        self,
        context: ProjectContext,
        control: &dyn WorkControl,
    ) -> Result<PreparedContextRefresh, WorkbenchError> {
        let mut document = self.base.projection_document();
        document.projections = self.projections.restore();
        document.refresh_project_with_control(context, control)?;
        Ok(PreparedContextRefresh {
            base: self.base,
            document,
            accepted: false,
        })
    }
}
