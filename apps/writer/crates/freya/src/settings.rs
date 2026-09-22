//! Settings visibly separate personal presentation from project-owned content.
use crate::design::tokens as t;
use crate::{
    editing::{Writer, editor_data},
    project::ProjectFiles,
};
use freya::{code_editor::*, prelude::*};
mod personal;
mod typography;

pub(super) fn render(writer: Writer, files: State<Option<ProjectFiles>>) -> Element {
    let mut project_tab = use_state(|| false);
    let mut opened = use_state(|| false);
    let mut project = use_state(|| None::<recite_config::ProjectSettings>);
    let mut draft = use_state(|| editor_data("", false, writer.dark));
    let mut error = use_state(String::new);
    let previous_focus = use_state(|| *Platform::get().focused_accessibility_id.peek());
    let editor_id = use_a11y();
    let source_viewport = crate::source_editor::EditorViewport::new();
    let config_id = use_a11y();
    let config_visible = use_state(|| false);
    let ids = [
        use_a11y(),
        use_a11y(),
        use_a11y(),
        use_a11y(),
        use_a11y(),
        use_a11y(),
        use_a11y(),
        use_a11y(),
        use_a11y(),
        use_a11y(),
    ];
    let option_ids = [
        [ids[2], use_a11y(), use_a11y()],
        [ids[3], use_a11y(), use_a11y()],
        [ids[4], use_a11y(), use_a11y()],
        [use_a11y(), use_a11y(), use_a11y()],
    ];
    let size_ids = [
        use_a11y(),
        use_a11y(),
        use_a11y(),
        use_a11y(),
        use_a11y(),
        use_a11y(),
    ];
    let preferences = writer.preferences;
    let mut visible = writer.settings_open;
    use_after_side_effect(move || {
        if *visible.read() {
            ids[0].request_focus();
        }
    });
    let mut previous_focus = previous_focus;
    let mut close = move || {
        visible.set(false);
        previous_focus.peek().request_focus();
    };
    if !*visible.read() {
        opened.set_if_modified(false);
        return rect().into_element();
    }
    if !*opened.peek() {
        previous_focus.set(*Platform::get().focused_accessibility_id.peek());
        project.set(None);
        if let Some(files) = files.peek().as_ref() {
            match recite_config::ProjectSettings::open(files.root()) {
                Ok(settings) => {
                    draft.set(editor_data(settings.source(), false, writer.dark));
                    project.set(Some(settings));
                    error.set(String::new());
                }
                Err(e) => error.set(project_error(e)),
            }
        }
        opened.set(true);
    }
    let on_project = *project_tab.read();
    let tab_order: Vec<_> = if on_project && project.peek().is_none() {
        vec![ids[0], ids[1], ids[9]]
    } else if on_project {
        vec![ids[0], ids[1], editor_id, ids[9], ids[8]]
    } else {
        let config = &preferences.read().config;
        let selected = [
            usize::from(config.writer.theme == recite_config::WriterTheme::Dark),
            usize::from(config.ui.keymap == recite_config::Keymap::Vim),
            match config.writer.view {
                recite_config::WriterView::Script => 0,
                recite_config::WriterView::Map => 1,
                recite_config::WriterView::Source => 2,
            },
            usize::from(config.writer.pane_side == recite_config::WriterPaneSide::Right),
        ];
        vec![
            ids[0],
            ids[1],
            option_ids[0][selected[0]],
            option_ids[1][selected[1]],
            option_ids[2][selected[2]],
            option_ids[3][selected[3]],
            size_ids[0],
            size_ids[1],
            size_ids[2],
            size_ids[3],
            size_ids[4],
            size_ids[5],
            ids[5],
            ids[6],
            ids[7],
            config_id,
            ids[9],
        ]
    };
    let mut content = rect().width(Size::fill()).spacing(t::SPACE_MD).child(
        rect()
            .horizontal()
            .spacing(t::SPACE_SM)
            .child(
                crate::design::Button::new()
                    .flat()
                    .selected(!on_project)
                    .a11y_id(ids[0])
                    .named(crate::messages::text(
                        crate::messages::MsgId::WriterGuiUserPreferences,
                    ))
                    .on_press(move |_| project_tab.set(false))
                    .child(crate::messages::text(
                        crate::messages::MsgId::WriterGuiUserPreferences,
                    )),
            )
            .child(
                crate::design::Button::new()
                    .flat()
                    .selected(on_project)
                    .a11y_id(ids[1])
                    .named(crate::messages::text(
                        crate::messages::MsgId::WriterGuiProjectSettings,
                    ))
                    .on_press(move |_| project_tab.set(true))
                    .child(crate::messages::text(
                        crate::messages::MsgId::WriterGuiProjectSettings,
                    )),
            ),
    );
    let mut primary = crate::design::SubmitAction {
        id: ids[9],
        caption: "Done".into(),
        enabled: true,
        action: EventHandler::new(move |()| close()),
    };
    if on_project {
        if let Some(settings) = project.read().as_ref() {
            content = content
                .child(
                    label()
                        .text(settings.path().display().to_string())
                        .font_size(t::small()),
                )
                .child(
                    label()
                        .text(crate::messages::text(
                            crate::messages::MsgId::WriterGuiProjectSettingsHint,
                        ))
                        .font_size(t::small()),
                )
                .child(
                    rect()
                        .height(Size::px(340.))
                        .child(crate::source_editor::EditorSurface {
                            editor: draft,
                            viewport: source_viewport,
                            id: editor_id,
                            size: t::body(),
                            content: CodeEditor::new(draft, editor_id)
                                .scroll_controller(source_viewport.scroll)
                                .font_family("monospace")
                                .font_size(t::body())
                                .gutter(false)
                                .on_pre_key_down(|e: Event<KeyboardEventData>| {
                                    if crate::design::keyboard::submit_key(&e)
                                        || matches!(
                                            e.key,
                                            Key::Named(NamedKey::Tab | NamedKey::Escape)
                                        )
                                    {
                                        return false;
                                    }
                                    e.stop_propagation();
                                    true
                                })
                                .into_element(),
                        }),
                );
            primary = crate::design::SubmitAction {
                id: ids[8],
                caption: "Apply project changes".into(),
                enabled: true,
                action: EventHandler::new(move |()| {
                    let text = draft.peek().rope.to_string();
                    let result = project
                        .write()
                        .as_mut()
                        .ok_or("No project".into())
                        .and_then(|p| p.save(&text).map_err(project_error));
                    match result {
                        Ok(()) => {
                            let mut model = writer.buffers.model;
                            let mut files = files;
                            if let (Some(files), Ok(model)) =
                                (files.write().as_mut(), model.write().as_mut())
                                && let Err(e) = files.refresh(model)
                            {
                                error.set(format!("Settings saved; project refresh failed: {e}"));
                                return;
                            }
                            error.set("Project settings saved.".into());
                        }
                        Err(e) => error.set(e),
                    }
                }),
            };
        } else {
            content = content.child(label().text(crate::messages::text(
                crate::messages::MsgId::WriterGuiOpenAProjectToEditItsSettings,
            )));
        }
    } else {
        content = content.child(personal::render(
            writer,
            &ids[2..8],
            option_ids,
            size_ids,
            error,
            config_id,
            config_visible,
        ));
    }
    if !error.read().is_empty() {
        content = content.child(label().text(error.read().clone()));
    }
    if let Some(e) = preferences.read().error.clone() {
        content = content.child(label().text(e));
    }
    let secondary = if on_project && project.peek().is_some() {
        action(ids[9], "Close settings", close)
    } else {
        rect().into_element()
    };
    crate::design::Dialog {
        dismissal_only: !on_project || project.peek().is_none(),
        primary,
        title: "Settings".into(),
        reduced_motion: preferences.read().config.writer.reduced_motion,
        close: EventHandler::new(move |()| close()),
        content: content.into_element(),
        focus_order: tab_order,
        actions: secondary,
    }
    .into_element()
}

pub(super) fn action(
    id: AccessibilityId,
    name: impl Into<String>,
    mut action: impl FnMut() + 'static,
) -> Element {
    let name = name.into();
    crate::design::Button::new()
        .a11y_id(id)
        .named(name.clone())
        .on_press(move |_| action())
        .child(label().text(name))
        .into_element()
}

fn project_error(error: recite_config::ProjectSettingsError) -> String {
    match error {
        recite_config::ProjectSettingsError::Validation(diagnostics) => {
            crate::project_context::diagnostic_messages(&diagnostics)
        }
        other => other.to_string(),
    }
}
