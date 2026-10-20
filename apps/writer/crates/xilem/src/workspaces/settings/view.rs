use super::*;
use crate::{
    attention::{Pane, Region},
    presentation::{label, paragraph, text_button},
};
use recite_writer_session::project::settings::SettingsAction;
use xilem::{
    WidgetView,
    masonry::{layout::AsUnit, properties::TextAreaLabel},
    style::Style,
    view::{CrossAxisAlignment, flex_col, flex_row, sized_box, text_input},
};

pub(in crate::workspaces) fn panel(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    flex_col((
        label(scale, "Project settings").text_size(28. * scale),
        paragraph(scale, "Edit a separate TOML draft in your IDE, then Import and Apply. Your manuscript stays open.", 640.),
        state(writer),
        feedback(writer),
        actions(writer),
        import(writer),
        preview(writer),
    )).cross_axis_alignment(CrossAxisAlignment::Start).gap(12.px()).prop(Region(Some(Pane::Settings)))
}
fn state(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let session = writer.project_settings()?;
    let message = if session.requires_reload() {
        "The saved settings baseline is unconfirmed. Open the saved manifest to inspect or repair it, then Reload saved settings before another Apply."
    } else if session.reviewed_source().is_some() {
        "Validated review ready. Apply writes these exact contents; edit again with another Import."
    } else {
        "Saved settings baseline. Export a draft to begin editing."
    };
    Some(paragraph(writer.ui_scale(), message, 640.))
}
fn actions(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let blocked = writer.project_action_blocked();
    let session = writer.project_settings();
    let busy = session.is_some_and(Session::busy);
    let scale = writer.ui_scale();
    flex_col((
        flex_row((
            text_button(scale, "Export settings draft", |writer: &mut Writer| {
                writer.project_settings_action(SettingsAction::ExportDraft)
            })
            .disabled(blocked || session.is_none()),
            text_button(
                scale,
                "Open settings draft in editor",
                Writer::open_settings_draft,
            )
            .disabled(
                blocked || writer.external.busy() || writer.workspaces.settings.artifact.is_none(),
            ),
            text_button(
                scale,
                "Apply reviewed settings",
                Writer::apply_project_settings,
            )
            .disabled(
                blocked
                    || session.is_none_or(|session| {
                        session.reviewed_source().is_none() || session.requires_reload()
                    }),
            ),
        ))
        .gap(8.px()),
        flex_row((
            text_button(
                scale,
                "Open saved manifest in editor",
                Writer::open_project_manifest,
            )
            .disabled(blocked || writer.external.busy() || session.is_none()),
            text_button(scale, "Reload saved settings", |writer: &mut Writer| {
                writer.project_settings_action(SettingsAction::Reload)
            })
            .disabled(blocked || session.is_none()),
            busy.then(|| {
                text_button(
                    scale,
                    "Cancel settings action",
                    Writer::cancel_project_settings,
                )
            }),
        ))
        .gap(8.px()),
        writer
            .session
            .project()
            .filter(|project| project.files().configuration_refresh_required())
            .map(|_| {
                text_button(
                    scale,
                    "Retry settings analysis refresh",
                    Writer::refresh_project_context,
                )
                .disabled(blocked)
            }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}
fn import(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let panel = &writer.workspaces.settings;
    let blocked = writer.project_action_blocked();
    let scale = writer.ui_scale();
    flex_col((
        writer.project_settings().map(|session| {
            paragraph(
                scale,
                format!("Saved manifest: {}", session.path().display()),
                640.,
            )
        }),
        panel
            .artifact
            .as_ref()
            .map(|path| paragraph(scale, format!("Editable draft: {}", path.display()), 640.)),
        label(scale, "Settings draft path"),
        text_input(panel.import.clone(), |writer: &mut Writer, path| {
            writer.workspaces.settings.set_import_path(path)
        })
        .placeholder("Path to the edited TOML draft")
        .text_size(16. * scale)
        .disabled(blocked)
        .prop(TextAreaLabel::new("Settings draft path")),
        text_button(
            scale,
            "Import settings draft",
            Writer::import_project_settings,
        )
        .disabled(blocked || panel.import_path().is_empty()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}
fn feedback(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let panel = &writer.workspaces.settings;
    let page = panel.feedback.get(panel.feedback_page)?;
    Some(
        flex_col((
            paragraph(writer.ui_scale(), page.clone(), 640.),
            (panel.feedback.len() > 1).then(|| {
                flex_row((
                    text_button(
                        writer.ui_scale(),
                        "Previous settings report",
                        |writer: &mut Writer| {
                            writer.workspaces.settings.feedback_page =
                                writer.workspaces.settings.feedback_page.saturating_sub(1)
                        },
                    )
                    .disabled(panel.feedback_page == 0),
                    text_button(
                        writer.ui_scale(),
                        "More settings report",
                        |writer: &mut Writer| writer.workspaces.settings.feedback_page += 1,
                    )
                    .disabled(panel.feedback_page + 1 >= panel.feedback.len()),
                ))
            }),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
    )
}
fn preview(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let panel = &writer.workspaces.settings;
    let scale = writer.ui_scale();
    flex_col((
        text_button(
            scale,
            if panel.preview_open {
                "Hide settings preview"
            } else {
                "Show settings preview"
            },
            |writer: &mut Writer| writer.workspaces.settings.toggle_preview(),
        ),
        panel
            .preview
            .get(panel.page)
            .filter(|_| panel.preview_open)
            .map(|page| {
                sized_box(
                    flex_col((
                        writer
                            .project_settings()
                            .and_then(Session::reviewed_path)
                            .map(|path| {
                                paragraph(scale, format!("Reviewed from: {}", path.display()), 640.)
                            }),
                        label(
                            scale,
                            format!(
                                "Read-only TOML preview · part {} of {}",
                                panel.page + 1,
                                panel.preview.len()
                            ),
                        ),
                        flex_row((
                            text_button(
                                scale,
                                "Previous settings preview",
                                |writer: &mut Writer| {
                                    writer.workspaces.settings.page =
                                        writer.workspaces.settings.page.saturating_sub(1)
                                },
                            )
                            .disabled(panel.page == 0),
                            text_button(scale, "More settings preview", |writer: &mut Writer| {
                                writer.workspaces.settings.page += 1
                            })
                            .disabled(panel.page + 1 >= panel.preview.len()),
                        )),
                        paragraph(scale, page.clone(), 640.),
                    ))
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .gap(8.px()),
                )
                .padding(12.px())
                .background_color(writer.palette.inset)
            }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}
