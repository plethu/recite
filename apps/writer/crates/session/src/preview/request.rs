//! One immutable trial capture, shared by native adapters.
use super::{Inputs, capture_provenance, catalogue_stale};
use crate::{
    localisation::catalogue::{Catalogue, PreviewInput},
    messages::{MsgId, text},
};
use recite_compiler::authoring::WorkControl;
use recite_writer_model::{PreparedPreview, PreviewProvenance, PreviewRequest, View, Workbench};

pub struct Request {
    source: PreviewRequest,
    view: View,
    inputs: Inputs,
    catalogue: Option<PreviewInput>,
    provenance: PreviewProvenance,
}

impl Request {
    /// Capture on the UI thread; catalogue I/O and compilation happen in `prepare`.
    pub fn capture(
        model: &Workbench,
        inputs: Inputs,
        catalogue: Option<&Catalogue>,
    ) -> Result<Self, String> {
        let source = model.preview_request().map_err(|error| error.to_string())?;
        let captured = catalogue
            .filter(|_| !inputs.locale.is_empty())
            .map(|catalogue| catalogue.preview_input(inputs.include_drafts));
        let locale = if inputs.locale.is_empty() {
            text(MsgId::WriterSourceOnly)
        } else {
            inputs.locale.clone()
        };
        let selection = if inputs.variant.trim().is_empty() {
            locale
        } else {
            format!(
                "{locale} · {}: {}",
                text(MsgId::WriterWording),
                inputs.variant.trim()
            )
        };
        let provenance = capture_provenance(
            format!(
                "{} · {} · {} {}",
                selection,
                text(if inputs.include_drafts {
                    MsgId::WriterTrialDrafts
                } else {
                    MsgId::WriterTrialSaved
                }),
                text(MsgId::WriterSourceRevision),
                model.document().revision()
            ),
            !inputs.locale.is_empty(),
            catalogue,
        );
        Ok(Self {
            source,
            view: model.view().clone(),
            inputs,
            catalogue: captured,
            provenance,
        })
    }

    pub fn is_current(&self, model: &Workbench, catalogue: Option<&Catalogue>) -> bool {
        model.view() == &self.view
            && !catalogue_stale(&self.provenance, catalogue)
            && self.source.is_current(model)
    }

    /// Call from a worker. A compiler phase may finish after cancellation.
    pub fn prepare(&self, control: &dyn WorkControl) -> Result<Box<PreparedPreview>, String> {
        control.checkpoint().map_err(|error| error.to_string())?;
        let mut setup = self.inputs.prepare(&self.source, self.catalogue.as_ref())?;
        setup.provenance = Some(self.provenance.clone());
        control.checkpoint().map_err(|error| error.to_string())?;
        self.source
            .prepare(setup)
            .map_err(|error| error.to_string())
    }
}
