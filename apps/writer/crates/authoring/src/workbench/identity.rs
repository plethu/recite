//! Queued UI choices check selection freshness without retaining analysis payloads.
use super::{SourceValidation, View, Workbench};
use crate::{AnalysisStamp, DraftSnapshot};

/// Selection and draft authority, without a strong compiler or project graph.
#[derive(Clone)]
pub struct NavigationStamp {
    analysis: AnalysisStamp,
    pub(super) view: View,
    pub(super) script: Option<String>,
    pub(super) draft: DraftSnapshot,
    pub(super) loaded: DraftSnapshot,
    pub(super) dirty: bool,
    pub(super) source_validation: SourceValidation,
}
impl Workbench {
    pub fn navigation_stamp(&self) -> NavigationStamp {
        NavigationStamp {
            analysis: self.document.analysis_stamp(),
            view: self.view.clone(),
            script: self.script_selection.clone(),
            draft: self.draft.clone(),
            loaded: self.loaded.clone(),
            dirty: self.dirty,
            source_validation: self.source_validation,
        }
    }
}
impl NavigationStamp {
    pub fn is_current(&self, model: &Workbench) -> bool {
        self.analysis.is_current(model.document())
            && self.view == model.view
            && self.script == model.script_selection
            && self.draft == model.draft
            && self.loaded == model.loaded
            && self.dirty == model.dirty
            && self.source_validation == model.source_validation
    }
}
