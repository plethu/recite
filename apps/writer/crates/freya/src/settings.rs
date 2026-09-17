//! Settings visibly separate personal presentation from project-owned content.
use crate::design::tokens as t;
use crate::{
    editing::{Writer, editor_data},
    project::ProjectFiles,
};
use freya::{code_editor::*, prelude::*};
mod personal;

pub(super) fn render(writer: Writer, files: State<Option<ProjectFiles>>) -> Element {
    let mut project_tab = use_state(|| false);
    let mut opened = use_state(|| false);
    let mut project = use_state(|| None::<recite_config::ProjectSettings>);
    let mut draft = use_state(|| editor_data("", false, writer.dark));
    let mut error = use_state(String::new);
    let previous_focus = use_state(|| *Platform::get().focused_accessibility_id.peek());
    let editor_id = use_a11y();
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
        [ids[2], use_a11y()],
        [ids[3], use_a11y()],
        [ids[4], use_a11y()],
        [use_a11y(), use_a11y()],
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
            usize::from(config.writer.view == recite_config::WriterView::Source),
            usize::from(config.writer.pane_side == recite_config::WriterPaneSide::Right),
        ];
        vec![
            ids[0],
            ids[1],
            option_ids[0][selected[0]],
            option_ids[1][selected[1]],
            option_ids[2][selected[2]],
            option_ids[3][selected[3]],
            ids[5],
            ids[6],
            ids[7],
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
                    .named("User preferences")
                    .on_press(move |_| project_tab.set(false))
                    .child("User preferences"),
            )
            .child(
                crate::design::Button::new()
                    .flat()
                    .selected(on_project)
                    .a11y_id(ids[1])
                    .named("Project settings")
                    .on_press(move |_| project_tab.set(true))
                    .child("Project settings"),
            ),
    );
    let mut primary = crate::design::DialogAction {
        id: ids[9],
        caption: "Close settings".into(),
        enabled: true,
        action: EventHandler::new(move |()| close()),
    };
    if on_project {
        if let Some(settings) = project.read().as_ref() {
            content = content
                .child(
                    label()
                        .text(settings.path().display().to_string())
                        .font_size(t::TEXT_SMALL),
                )
                .child(
                    label()
                        .text("Project manifest · changes affect everyone using this project.")
                        .font_size(t::TEXT_SMALL),
                )
                .child(
                    rect().height(Size::px(340.)).child(
                        CodeEditor::new(draft, editor_id)
                            .font_family("monospace")
                            .font_size(t::TEXT_BODY)
                            .gutter(true)
                            .on_pre_key_down(|e: Event<KeyboardEventData>| {
                                if crate::design::keyboard::submit_key(&e)
                                    || matches!(e.key, Key::Named(NamedKey::Tab | NamedKey::Escape))
                                {
                                    return false;
                                }
                                e.stop_propagation();
                                true
                            }),
                    ),
                );
            primary = crate::design::DialogAction {
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
            content = content.child(label().text("Open a project to edit its settings."));
        }
    } else {
        content = content.child(personal::render(writer, &ids[2..8], option_ids, error));
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
