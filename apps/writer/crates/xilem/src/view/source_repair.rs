//! Source drafts are repaired in a real editor, then imported through checked Apply.
use super::*;
use xilem::masonry::properties::TextAreaLabel;

pub(super) fn panel(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let busy = state.phase != Phase::Writing || state.navigation.busy() || state.work.busy();
    let project = state.session.project();
    let exported = state.repair.artifact.as_ref().is_some_and(|artifact| {
        project.is_some_and(|project| project.current().path() == artifact.document)
    });
    flex_col((
            label(scale, "Repair source draft").text_size(22. * scale),
            label(scale, if state.source_repair_pending() {
                "Import is required before Save"
            } else if state.session.current_dirty() {
                "Repaired source accepted · Unsaved"
            } else { "Accepted source · Saved" }),
            paragraph(scale, "Export the staged draft and its recovery backup, then repair the text in your editor. Import validates the result before accepting it. Save file writes accepted source to the project.", state.script_width()),
            text_button(scale, "Export draft and open editor", Writer::export_source_repair)
                .disabled(busy || project.is_none() || state.external.busy()),
            exported.then(|| text_button(scale, "Open exported source", Writer::open_exported_source)
                .disabled(busy || state.external.busy())),
            label(scale, "Repaired source file"),
            sized_box(keyed(
                state.session.model().document().key().clone(),
                text_input(state.repair_path().to_owned(), Writer::edit_repair_path)
                .text_size(14. * scale).placeholder("Choose the text file repaired in your editor")
                .on_enter(|state: &mut Writer, _| state.import_source_repair())
                .disabled(busy).prop(TextAreaLabel::new("Repaired source file")),
            )).width(state.script_width().px()),
            flex_row((
                text_button(scale, "Choose file…", Writer::choose_source_repair).disabled(busy),
                text_button(scale, "Import repaired source", Writer::import_source_repair)
                    .disabled(busy || state.repair_path().trim().is_empty()),
            )).gap(8.px()),
        )).cross_axis_alignment(CrossAxisAlignment::Start).gap(8.px())
}
