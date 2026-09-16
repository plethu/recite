//! Settings visibly separate personal presentation from project-owned content.
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
        vec![ids[0], ids[1], editor_id, ids[8], ids[9]]
    } else {
        ids.into_iter().filter(|id| *id != ids[8]).collect()
    };
    let mut content = rect()
        .width(Size::fill())
        .spacing(12.)
        .on_global_key_down(move |event: Event<KeyboardEventData>| {
            if event.key == Key::Named(NamedKey::Tab) {
                let current = *Platform::get().focused_accessibility_id.peek();
                let index = tab_order.iter().position(|id| *id == current).unwrap_or(0);
                let step = if event.modifiers.contains(Modifiers::SHIFT) {
                    tab_order.len() - 1
                } else {
                    1
                };
                tab_order[(index + step) % tab_order.len()].request_focus();
                event.stop_propagation();
                event.prevent_default();
            }
        })
        .child(PopupTitle::new("Settings".to_owned()))
        .child(
            rect()
                .horizontal()
                .spacing(8.)
                .child(action(ids[0], "User preferences", move || {
                    project_tab.set(false)
                }))
                .child(action(ids[1], "Project settings", move || {
                    project_tab.set(true)
                })),
        );
    if on_project {
        if let Some(settings) = project.read().as_ref() {
            content = content
                .child(
                    label()
                        .text(settings.path().display().to_string())
                        .font_size(12.),
                )
                .child(
                    label()
                        .text("Project manifest · changes affect everyone using this project.")
                        .font_size(12.),
                )
                .child(
                    rect().height(Size::px(340.)).child(
                        CodeEditor::new(draft, editor_id)
                            .font_family("monospace")
                            .font_size(14.)
                            .gutter(true)
                            .on_pre_key_down(|e: Event<KeyboardEventData>| {
                                if matches!(e.key, Key::Named(NamedKey::Tab | NamedKey::Escape)) {
                                    return false;
                                }
                                e.stop_propagation();
                                true
                            }),
                    ),
                )
                .child(action(ids[8], "Apply project changes", move || {
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
                }));
        } else {
            content = content.child(label().text("Open a project to edit its settings."));
        }
    } else {
        content = content.child(personal::render(writer, &ids[2..8], error));
    }
    if !error.read().is_empty() {
        content = content.child(label().text(error.read().clone()));
    }
    if let Some(e) = preferences.read().error.clone() {
        content = content.child(label().text(e));
    }
    Popup::new()
        .on_close_request(move |_| close())
        .child(content.child(action(ids[9], "Close settings", close)))
        .into_element()
}

pub(super) fn action(
    id: AccessibilityId,
    name: impl Into<String>,
    mut action: impl FnMut() + 'static,
) -> Element {
    let name = name.into();
    rect()
        .a11y_id(id)
        .a11y_focusable(true)
        .a11y_role(AccessibilityRole::Button)
        .a11y_alt(name.clone())
        .padding(8.)
        .cursor(CursorIcon::Pointer)
        .border(
            Border::new()
                .width(if id.is_focused() { 2. } else { 1. })
                .fill((120, 130, 115)),
        )
        .on_all_press(move |e: Event<PressEventData>| {
            e.stop_propagation();
            id.request_focus();
            action();
        })
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
