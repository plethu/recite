//! Prepare destination selection against immutable analysis; retain live history and preview.
use super::{NavigationStamp, Workbench, WorkbenchError};
use crate::DocumentVersion;
use crate::{CompletionSnapshot, EditError, ProjectContext, projection_cache::CachedProjections};
use recite_compiler::authoring::{AuthoringError, WorkControl};

pub struct NavigationRequest {
    base: CompletionSnapshot,
    projections: CachedProjections,
    selection: NavigationStamp,
}
pub struct PreparedNavigation {
    request: NavigationRequest,
    model: Workbench,
    accepted: bool,
    rules_revision: Option<DocumentVersion>,
}
impl Workbench {
    /// Capture a destination without copying its history, preview or projection arrays.
    pub fn navigation_request(&self) -> NavigationRequest {
        NavigationRequest {
            base: self.document.completion_snapshot(),
            projections: self.document.projections.capture_local(),
            selection: self.navigation_stamp(),
        }
    }
    /// Publish prepared context/selection once; keep the receipt for worker retirement.
    pub fn accept_navigation(
        &mut self,
        prepared: &mut PreparedNavigation,
    ) -> Result<(), WorkbenchError> {
        if !prepared.is_current(self) {
            return Err(EditError::Stale.into());
        }
        self.document
            .accept_navigation_analysis(&prepared.request.base, &mut prepared.model.document)?;
        std::mem::swap(&mut self.view, &mut prepared.model.view);
        std::mem::swap(
            &mut self.script_selection,
            &mut prepared.model.script_selection,
        );
        std::mem::swap(&mut self.draft, &mut prepared.model.draft);
        std::mem::swap(&mut self.loaded, &mut prepared.model.loaded);
        self.dirty = prepared.model.dirty;
        self.source_validation = prepared.model.source_validation;
        self.draft_revision = self.document.revision();
        prepared.accepted = true;
        Ok(())
    }
}
impl NavigationRequest {
    pub fn is_current(&self, model: &Workbench) -> bool {
        self.selection.is_current(model)
    }
    pub fn prepare(
        self,
        context: Option<ProjectContext>,
        control: &dyn WorkControl,
    ) -> Result<PreparedNavigation, WorkbenchError> {
        control
            .checkpoint()
            .map_err(AuthoringError::from)
            .map_err(EditError::from)?;
        let mut document = self.base.projection_document();
        document.projections = self.projections.restore();
        if let Some(context) = context {
            document.refresh_project_with_control(context, control)?;
        }
        let rules_revision = self.base.rules_revision(&document);
        let model = Workbench {
            draft_revision: document.revision(),
            document,
            view: self.selection.view.clone(),
            script_selection: self.selection.script.clone(),
            draft: self.selection.draft.clone(),
            loaded: self.selection.loaded.clone(),
            dirty: self.selection.dirty,
            preview: None,
            page: None,
            source_validation: self.selection.source_validation,
        };
        Ok(PreparedNavigation {
            request: self,
            model,
            accepted: false,
            rules_revision,
        })
    }
}
impl PreparedNavigation {
    pub(super) fn into_parts(self) -> (NavigationRequest, Workbench) {
        (self.request, self.model)
    }
    /// Preserve staged Rules across sibling edits only when their source and schema stayed equal.
    pub fn refresh_rules(&self, rules: &mut crate::ReplyRules, document: &crate::Document) -> bool {
        self.accepted
            && self.rules_revision == Some(document.revision())
            && rules.refresh_revision(&self.request.base.projection_document(), document)
    }
    pub fn is_current(&self, model: &Workbench) -> bool {
        !self.accepted
            && self.request.is_current(model)
            && model
                .document
                .navigation_analysis_is_current(&self.request.base, &self.model.document)
    }

    pub fn model(&self) -> &Workbench {
        &self.model
    }
    /// Destination validation uses the same selection methods as synchronous navigation.
    pub fn model_mut(&mut self) -> &mut Workbench {
        &mut self.model
    }
}
