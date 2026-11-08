//! Capture document and draft authority for worker publication.
use crate::WindowKey;
use crate::{Phase, Writer};
use recite_writer_model::{AnalysisStamp, RecoveredDraft};

pub struct Origin {
    pub window: WindowKey,
    pub source: AnalysisStamp,
    pub draft: RecoveredDraft,
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
    pub fn presentation_is_current(&self, writer: &Writer) -> bool {
        self.window == writer.window
            && self.source.is_current(writer.session.model().document())
            && self.draft.view() == writer.session.model().view()
            && writer.phase == Phase::Writing
    }
    pub fn is_current(&self, writer: &Writer) -> bool {
        self.presentation_is_current(writer) && self.draft == writer.session.model().recovery()
    }
}
