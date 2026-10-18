//! Prepare destination selection against immutable analysis; retain live history and preview.
use super::{View, Workbench, WorkbenchError};
use crate::{
    CompletionSnapshot, DraftSnapshot, EditError, ProjectContext,
    projection_cache::CachedProjections,
};
use recite_compiler::authoring::{AuthoringError, WorkControl};

pub struct NavigationRequest {
    base: CompletionSnapshot,
    projections: CachedProjections,
    view: View,
    script: Option<String>,
    draft: DraftSnapshot,
    loaded: DraftSnapshot,
    dirty: bool,
}
pub struct PreparedNavigation {
    request: NavigationRequest,
    model: Workbench,
    accepted: bool,
    rules_revision: Option<i64>,
}
impl Workbench {
    /// Capture a destination without copying its history, preview or projection arrays.
    pub fn navigation_request(&self) -> NavigationRequest {
        NavigationRequest {
            base: self.document.completion_snapshot(),
            projections: self.document.projections.capture_local(),
            view: self.view.clone(),
            script: self.script_selection.clone(),
            draft: self.draft.clone(),
            loaded: self.loaded.clone(),
            dirty: self.dirty,
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
        self.draft_revision = self.document.revision();
        prepared.accepted = true;
        Ok(())
    }
}
impl NavigationRequest {
    pub fn is_current(&self, model: &Workbench) -> bool {
        self.base.is_current(model.document())
            && self.view == model.view
            && self.script == model.script_selection
            && self.draft == model.draft
            && self.loaded == model.loaded
            && self.dirty == model.dirty
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
            view: self.view.clone(),
            script_selection: self.script.clone(),
            draft: self.draft.clone(),
            loaded: self.loaded.clone(),
            dirty: self.dirty,
            preview: None,
            page: None,
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
