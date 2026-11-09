//! Frontend views over `recite_writer_app::workspaces::declarations::producer`.
use super::*;
use recite_writer_session::declarations::DeclarationAction;

pub(crate) fn controls(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let session = writer.declaration_session();
    let registration = session.and_then(|session| session.registration.as_ref().ok()?.as_ref());
    let panel = &writer.workspaces.declarations.producer;
    flex_col((
        label(scale, "Schema producer").text_size(18. * scale),
        flex_row((
            text_button(
                scale,
                "Reload producer registration",
                |writer: &mut Writer| {
                    writer.declaration_action(DeclarationAction::ReloadRegistration)
                },
            )
            .disabled(writer.declaration_action_blocked()),
            text_button(scale, "Reload generated schema", |writer: &mut Writer| {
                writer.declaration_action(DeclarationAction::ReloadGenerated)
            })
            .disabled(writer.declaration_action_blocked()),
        )),
        label(
            scale,
            format!(
                "Producer command · part {} of {}",
                panel.page + 1,
                panel.pages.len()
            ),
        ),
        paragraph(
            scale,
            panel.pages.get(panel.page).cloned().unwrap_or_default(),
            640.,
        ),
        flex_row((
            text_button(scale, "Previous command detail", |writer: &mut Writer| {
                writer.workspaces.declarations.producer.page = writer
                    .workspaces
                    .declarations
                    .producer
                    .page
                    .saturating_sub(1)
            })
            .disabled(panel.page == 0),
            text_button(scale, "More command detail", |writer: &mut Writer| {
                writer.workspaces.declarations.producer.page += 1
            })
            .disabled(panel.page + 1 >= panel.pages.len()),
        )),
        flex_row((
            text_button(scale, "Regenerate schema", Writer::regenerate_schema).disabled(
                writer.declaration_action_blocked()
                    || registration.is_none()
                    || session.is_some_and(|session| session.dirty()),
            ),
            writer.workspaces.context_refresh.is_some().then(|| {
                text_button(
                    scale,
                    "Retry schema analysis",
                    Writer::refresh_project_context,
                )
                .disabled(writer.workspaces.tools.busy() || writer.work.busy())
            }),
        )),
        super::outcome::panel(writer),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
}
