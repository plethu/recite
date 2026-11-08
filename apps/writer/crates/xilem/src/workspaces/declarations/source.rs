//! Frontend views over `recite_writer_app::workspaces::declarations::source`.
pub(crate) use recite_writer_app::workspaces::declarations::source::*;
mod actions;
mod preview;
mod repair;
use super::*;
use recite_writer_session::declarations::DeclarationAction;

pub(crate) fn panel(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    flex_col((
        binding(writer),
        bound_source(writer),
        writer
            .declaration_session()
            .is_some_and(|session| session.busy() && !session.generation_active())
            .then(|| {
                text_button(
                    scale,
                    "Cancel declaration action",
                    Writer::cancel_declaration_action,
                )
            }),
        super::outcome::panel(writer),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
}

fn binding(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    if writer
        .declaration_session()
        .is_none_or(|session| session.source.is_some())
    {
        return None;
    }
    let scale = writer.ui_scale();
    let blocked = writer.declaration_action_blocked();
    Some(flex_col((
        label(scale, "Standalone declaration source (TOML)").text_size(18. * scale),
        paragraph(scale, "Bind your standalone TOML to edit it in your preferred editor. Writer validates explicit reloads and imports, then generates read-only JSON.", 640.),
        label(scale, "Declaration source path"),
        text_input(writer.workspaces.declarations.source.path.clone(), |writer: &mut Writer, path| writer.workspaces.declarations.source.path = path)
            .placeholder("Absolute path or path relative to this project").text_size(16. * scale)
            .disabled(blocked).prop(TextAreaLabel::new("Declaration source path")),
        text_button(scale, "Bind standalone source", Writer::bind_declaration_source)
            .disabled(blocked || writer.workspaces.declarations.source.path.trim().is_empty()),
    )).cross_axis_alignment(CrossAxisAlignment::Start).gap(8.px()))
}

fn bound_source(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let session = writer.declaration_session()?;
    let source = session.source.as_ref()?;
    let panel = &writer.workspaces.declarations.source;
    let scale = writer.ui_scale();
    Some(
        flex_col((
            label(
                scale,
                if session.dirty() {
                    "Standalone TOML · draft or recovery needs saving"
                } else {
                    "Standalone TOML · source saved"
                },
            )
            .text_size(18. * scale),
            paragraph(scale, panel.status_message(), 640.),
            actions::primary(writer),
            paragraph(
                scale,
                format!("Saved source: {}", source.path.display()),
                640.,
            ),
            actions::recovery(writer),
            repair::panel(writer),
            preview::panel(writer),
            panel.archive.as_ref().map(|path| {
                paragraph(
                    scale,
                    format!("Previous draft preserved at {}", path.display()),
                    640.,
                )
            }),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(12.px()),
    )
}
