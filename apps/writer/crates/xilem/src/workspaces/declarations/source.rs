//! Frontend views over `recite_writer_app::workspaces::declarations::source`.
use crate::presentation::print::section_heading;
use crate::presentation::space;
use crate::presentation::tokens::size;
use crate::presentation::{Type, Typeset};
pub(crate) use recite_writer_app::workspaces::declarations::source::*;
mod actions;
pub(super) use actions::apply;
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
    .gap(space::TEXT.px())
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
    Some(
        flex_col((
            section_heading(scale, "Standalone declaration source (TOML)"),
            paragraph(
                scale,
                "Bind your standalone TOML to edit it in your preferred \
                 editor. Writer validates explicit reloads and imports, then \
                 generates read-only JSON.",
                size::MEASURE,
            ),
            label(scale, "Declaration source path"),
            text_input(
                writer.workspaces.declarations.source.path.clone(),
                |writer: &mut Writer, path| writer.workspaces.declarations.source.path = path,
            )
            .placeholder("Absolute path or path relative to this project")
            .typeset(Type::Body, scale)
            .disabled(blocked)
            .prop(TextAreaLabel::new("Declaration source path")),
            text_button(
                scale,
                "Bind standalone source",
                Writer::bind_declaration_source,
            )
            .disabled(blocked || writer.workspaces.declarations.source.path.trim().is_empty()),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::SIBLINGS.px()),
    )
}

fn bound_source(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let session = writer.declaration_session()?;
    let source = session.source.as_ref()?;
    let panel = &writer.workspaces.declarations.source;
    let scale = writer.ui_scale();
    Some(
        flex_col((
            section_heading(
                scale,
                if session.dirty() {
                    "Standalone TOML · draft or recovery needs saving"
                } else {
                    "Standalone TOML · source saved"
                },
            ),
            paragraph(scale, panel.status_message(), size::MEASURE),
            actions::primary(writer),
            paragraph(
                scale,
                format!("Saved source: {}", source.path.display()),
                size::MEASURE,
            ),
            actions::recovery(writer),
            repair::panel(writer),
            preview::panel(writer),
            panel.archive.as_ref().map(|path| {
                paragraph(
                    scale,
                    format!("Previous draft preserved at {}", path.display()),
                    size::MEASURE,
                )
            }),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::TEXT.px()),
    )
}
