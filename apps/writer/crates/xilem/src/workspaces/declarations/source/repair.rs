use super::*;
pub(super) fn panel(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let scale = writer.ui_scale();
    let panel = &writer.workspaces.declarations.source;
    if !panel.repair_open {
        return None;
    }
    let blocked = writer.declaration_action_blocked();
    Some(xilem::view::sized_box(flex_col((
        label(scale, "Declaration draft repair").text_size(18. * scale),
        paragraph(scale, "Export a separate TOML file for your editor, then import the repaired draft. The saved source stays unchanged until Save and generate.", 640.),
        text_button(scale, "Export declaration draft", |writer: &mut Writer| writer.source_action(SourceAction::ExportDraft)).disabled(blocked),
        panel.artifact.as_ref().map(|path| paragraph(scale, format!("Editable draft: {}", path.display()), 640.)),
        panel.artifact.as_ref().filter(|_| !matches!(panel.validation, Validation::Invalid(_))).map(|_| text_button(scale, "Open exported declaration draft in editor", Writer::open_declaration_draft).disabled(blocked || writer.external.busy())),
        label(scale, "Repaired declaration draft path"),
        text_input(panel.import.clone(), |writer: &mut Writer, path| writer.workspaces.declarations.source.import = path)
            .placeholder("Path to the repaired TOML artifact").text_size(16. * scale).disabled(blocked).prop(TextAreaLabel::new("Repaired declaration draft path")),
        text_button(scale, "Import repaired declaration draft", Writer::import_declaration_draft).disabled(blocked || panel.import.trim().is_empty()),
    )).cross_axis_alignment(CrossAxisAlignment::Start).gap(8.px())).padding(12.px()).background_color(writer.palette.surface))
}
