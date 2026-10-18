use super::*;
use recite_writer_model::{Document, Workbench};
#[test]
fn prepared_view_does_not_override_newer_drafts_or_departure() {
    let mut model = Workbench::from_document(
        Document::new(recite_writer_model::FIXTURE).unwrap(),
        Some(View::Source),
    )
    .unwrap();
    let request = model.document().projection_request();
    let intent = ViewIntent {
        origin: View::Source,
        pane: Pane::Map,
        view: recite_config::WriterView::Script,
        split: Some(true),
    };
    assert!(intent.is_current(&model, Pane::Map));
    assert!(!intent.is_current(&model, Pane::Preview));
    model.set_draft(format!("{}\n# Later edit", model.document().source()));
    assert!(
        request.is_current(model.document()),
        "cache publication is still valid for accepted source"
    );
    assert!(
        !intent.is_current(&model, Pane::Map),
        "navigation must preserve the live draft"
    );
    model.discard();
    model.inspect_block("which_way").unwrap();
    assert!(!intent.is_current(&model, Pane::Map));
}
