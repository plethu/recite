//! Capture document and draft authority for worker publication.
pub use crate::WindowKey;
pub use crate::{Phase, Writer};
use recite_writer_model::{AnalysisStamp, RecoveredDraft};

pub(crate) struct Origin {
    pub(crate) window: WindowKey,
    pub(super) source: AnalysisStamp,
    pub(super) draft: RecoveredDraft,
}
impl Origin {
    pub fn capture(writer: &Writer) -> Self {
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
    pub fn is_current(&self, writer: &Writer) -> bool {
        self.presentation_is_current(writer) && self.draft == writer.session.model().recovery()
    }
}
