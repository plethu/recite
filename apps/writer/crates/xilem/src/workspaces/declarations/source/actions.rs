use super::*;

pub fn primary(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let blocked = writer.declaration_action_blocked();
    let invalid = matches!(
        writer.workspaces.declarations.source.validation,
        Validation::Invalid(_)
    );
    flex_col((
        matches!(
            writer.workspaces.declarations.source.validation,
            Validation::Unchecked
        )
        .then(|| {
            text_button(
                scale,
                "Validate declaration draft",
                |writer: &mut Writer| writer.declaration_action(DeclarationAction::InspectSource),
            )
            .disabled(blocked)
        }),
        (invalid && writer.workspaces.declarations.source.artifact.is_none()).then(|| {
            text_button(scale, "Export declaration draft", |writer: &mut Writer| {
                writer.declaration_action(DeclarationAction::ExportDraft)
            })
            .disabled(blocked)
        }),
        (invalid && writer.workspaces.declarations.source.artifact.is_some()).then(|| {
            text_button(
                scale,
                "Open exported declaration draft in editor",
                Writer::open_declaration_draft,
            )
            .disabled(blocked || writer.external.busy())
        }),
        (!invalid).then(|| {
            text_button(
                scale,
                "Open saved declaration source in editor",
                Writer::open_declaration_source,
            )
            .disabled(blocked || writer.external.busy())
        }),
        (!invalid).then(|| {
            text_button(
                scale,
                "Reload declaration source (keep recovery copy)",
                |writer: &mut Writer| writer.declaration_action(DeclarationAction::ReloadSource),
            )
            .disabled(blocked)
        }),
        (!invalid).then(|| {
            text_button(
                scale,
                "Save and generate declarations",
                Writer::save_declarations,
            )
            .disabled(blocked)
        }),
        if let Validation::Invalid(error) = &writer.workspaces.declarations.source.validation {
            Some(paragraph(scale, error.clone(), 640.))
        } else {
            None
        },
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}
pub fn recovery(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let blocked = writer.declaration_action_blocked();
    let dirty = writer
        .declaration_session()
        .is_some_and(|session| session.dirty());
    flex_col((
        label(scale, "Draft recovery").text_size(18. * scale),
        matches!(
            writer.workspaces.declarations.source.validation,
            Validation::Invalid(_)
        )
        .then(|| {
            text_button(
                scale,
                "Reload declaration source (keep recovery copy)",
                |writer: &mut Writer| writer.declaration_action(DeclarationAction::ReloadSource),
            )
            .disabled(blocked)
        }),
        dirty.then(|| {
            text_button(scale, "Discard declaration draft", |writer: &mut Writer| {
                writer.declaration_action(DeclarationAction::Discard)
            })
            .disabled(blocked)
        }),
        text_button(
            scale,
            if writer.workspaces.declarations.source.repair_open {
                "Hide declaration draft repair"
            } else {
                "Repair or import a declaration draft"
            },
            |writer: &mut Writer| {
                writer.workspaces.declarations.source.repair_open =
                    !writer.workspaces.declarations.source.repair_open
            },
        ),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}
