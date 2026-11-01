mod preview;
use super::*;
use crate::{
    attention::{Pane, PaneEntry, Region},
    presentation::{label, paragraph, text_button},
};
use preview::preview;
use recite_writer_session::project::settings::SettingsAction;
use xilem::{
    WidgetView,
    masonry::{kurbo::Axis, layout::AsUnit, properties::TextAreaLabel},
    style::Style,
    view::{CrossAxisAlignment, flex_col, flex_row, sized_box, text_input},
};

pub(in crate::workspaces) fn panel(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let reload = writer
        .project_settings()
        .is_some_and(Session::requires_reload);
    flex_col((
        label(scale, "Project settings").text_size(24. * scale),
        writer.project_settings().map(|session| {
            paragraph(
                scale,
                format!("Manifest: {}", session.path().display()),
                writer.content_width(),
            )
        }),
        (!reload).then(|| {
            paragraph(
                scale,
                "Edit a TOML draft in your IDE, then import a validated review before applying.",
                writer.content_width(),
            )
        }),
        state(writer),
        reload.then(|| baseline(writer)),
        feedback(writer),
        actions(writer),
        import(writer),
        preview(writer),
        (!reload).then(|| baseline(writer)),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
    .prop(Region(Some(Pane::Settings)))
}
fn state(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let session = writer.project_settings()?;
    let message = if session.requires_reload() {
        "Saved settings need confirmation. Inspect the manifest, then Reload saved settings."
    } else if session.reviewed_source().is_some() {
        "3 · Review ready. Apply writes these exact contents; another Import replaces the review."
    } else if session.artifact().is_some() {
        "2 · Open the exported draft in your editor, then Import it for review."
    } else {
        "1 · Export a separate draft to begin. The saved manifest stays unchanged until Apply."
    };
    Some(paragraph(
        writer.ui_scale(),
        message,
        writer.content_width(),
    ))
}
fn actions(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let blocked = writer.project_action_blocked();
    let session = writer.project_settings();
    let busy = session.is_some_and(Session::busy);
    let reload = session.is_some_and(Session::requires_reload);
    let reviewed = session.is_some_and(|session| session.reviewed_source().is_some());
    let scale = writer.ui_scale();
    flex_col((
        flex_row((
            (!reviewed).then(|| {
                text_button(scale, "Export settings draft", |writer: &mut Writer| {
                    writer.project_settings_action(SettingsAction::ExportDraft)
                })
                .disabled(blocked || session.is_none())
            }),
            (!reviewed && writer.workspaces.settings.artifact.is_some()).then(|| {
                text_button(
                    scale,
                    "Open settings draft in editor",
                    Writer::open_settings_draft,
                )
                .disabled(
                    blocked
                        || writer.external.busy()
                        || writer.workspaces.settings.artifact.is_none(),
                )
            }),
            session
                .filter(|session| session.reviewed_source().is_some() && !session.requires_reload())
                .map(|_| {
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
                    )
                }),
        ))
        .direction(if writer.content_width() < 600. * f64::from(scale) {
            Axis::Vertical
        } else {
            Axis::Horizontal
        })
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
        busy.then(|| {
            text_button(
                scale,
                "Cancel settings action",
                Writer::cancel_project_settings,
            )
        }),
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
    .prop(PaneEntry(!reload))
}
fn baseline(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let blocked = writer.project_action_blocked();
    let session = writer.project_settings();
    flex_col((
        label(scale, "Saved manifest and recovery").text_size(18. * scale),
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
        ))
        .direction(if writer.content_width() < 600. * f64::from(scale) {
            Axis::Vertical
        } else {
            Axis::Horizontal
        })
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
    .prop(PaneEntry(session.is_some_and(Session::requires_reload)))
}
fn import(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let panel = &writer.workspaces.settings;
    let blocked = writer.project_action_blocked();
    let scale = writer.ui_scale();
    flex_col((
        label(scale, "Settings draft path"),
        text_input(panel.import.clone(), |writer: &mut Writer, path| {
            writer.workspaces.settings.set_import_path(path)
        })
        .placeholder("Path to the edited TOML draft")
        .text_size(16. * scale)
        .disabled(blocked)
        .width(writer.content_width().px())
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
            paragraph(
                writer.ui_scale(),
                page.clone(),
                writer.content_width().min(640.),
            ),
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
