//! Capture document and draft authority for worker publication.
use crate::{Phase, Writer};
use recite_writer_model::{AnalysisStamp, RecoveredDraft};
use xilem::WindowId;

pub(crate) struct Origin {
    pub(super) window: WindowId,
    pub(super) source: AnalysisStamp,
    pub(super) draft: RecoveredDraft,
}
impl Origin {
    pub(crate) fn capture(writer: &Writer) -> Self {
        let model = writer.session.model();
        Self {
            window: writer.window,
            source: model.document().analysis_stamp(),
            draft: model.recovery(),
        }
    }
    pub(super) fn presentation_is_current(&self, writer: &Writer) -> bool {
        self.window == writer.window
            && self.source.is_current(writer.session.model().document())
            && self.draft.view() == writer.session.model().view()
            && writer.phase == Phase::Writing
    }
    pub(crate) fn is_current(&self, writer: &Writer) -> bool {
        self.presentation_is_current(writer) && self.draft == writer.session.model().recovery()
    }
}
