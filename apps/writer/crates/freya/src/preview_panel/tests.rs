use super::{Snapshot, prepare::Inputs, request::Request};
use crate::localisation::catalogue::Catalogue;
use recite_writer_model::Workbench;

pub(super) fn capture(
    model: &Workbench,
    catalogue: Option<&Catalogue>,
    localised: bool,
) -> Request {
    Request {
        source: model.preview_request().unwrap(),
        view: model.view().clone(),
        snapshot: Snapshot::capture("captured settings".into(), localised, catalogue),
        inputs: Inputs {
            locale: String::new(),
            variant: String::new(),
            include_drafts: false,
            values: None,
            catalogue: None,
        },
    }
}
