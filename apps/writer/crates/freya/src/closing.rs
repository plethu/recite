//! One close policy for native window requests and application keyboard shortcuts.
use crate::{files::Buffers, project::ProjectFiles};
use freya::prelude::*;
use std::cell::RefCell;

thread_local! {
    static CLOSE: RefCell<Option<Box<dyn FnMut() -> CloseDecision>>> = RefCell::new(None);
}

pub fn request_close() -> CloseDecision {
    CLOSE.with(|handler| {
        handler
            .borrow_mut()
            .as_mut()
            .map_or(CloseDecision::KeepOpen, |handler| handler())
    })
}

fn close_window() {
    let platform = Platform::get();
    platform
        .clone()
        .with_window(None, move |window| platform.close_window(window.id()));
}

pub(super) fn keyboard(event: Event<KeyboardEventData>) {
    let quit = is_quit_key(&event);
    let escape = event.key == Key::Named(NamedKey::Escape)
        && event.modifiers.is_empty()
        && Platform::get().focused_accessibility_id.peek().0 == 0;
    if quit || escape {
        event.stop_propagation();
        event.prevent_default();
        if matches!(request_close(), CloseDecision::Close) {
            close_window();
        }
    }
}

pub(super) fn is_quit_key(event: &KeyboardEventData) -> bool {
    let modifier = if cfg!(target_os = "macos") {
        Modifiers::META
    } else {
        Modifiers::CONTROL
    };
    event.code == Code::KeyQ && event.modifiers == modifier
}

/// Text widgets otherwise consume shortcuts before global listeners see them.
pub(super) fn text_input_key(event: Event<KeyboardEventData>) -> bool {
    if crate::editing::is_workspace_key(&event) {
        return false;
    }
    if crate::editing::is_save_key(&event) {
        return false;
    }
    if is_quit_key(&event) {
        keyboard(event);
        return false;
    }
    match event.key {
        Key::Named(NamedKey::Tab) => false,
        Key::Named(NamedKey::Escape) | Key::Named(NamedKey::Shift) => true,
        _ => {
            event.stop_propagation();
            true
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Prompt {
    Exit,
    Unsaved,
}

pub(super) fn controls(
    buffers: Buffers,
    mut preferences: State<crate::preferences::Preferences>,
    mut files: State<Option<ProjectFiles>>,
) -> Element {
    let mut pending = use_state(|| None::<Prompt>);
    let mut failure = use_state(|| None::<String>);
    let mut dont_ask = use_state(|| false);
    let mut previous_focus = use_state(|| None::<AccessibilityId>);
    let actions = [use_a11y(), use_a11y(), use_a11y()];
    let platform = Platform::get();
    let mut cancel = move || {
        pending.set(None);
        dont_ask.set(false);
        if let Some(id) = *previous_focus.peek() {
            id.request_focus();
        }
    };
    use_after_side_effect(move || {
        if pending.read().is_some() {
            actions[0].request_focus();
        }
    });
    use_hook(move || {
        CLOSE.with(|handler| {
            *handler.borrow_mut() = Some(Box::new(move || {
                if pending.peek().is_some() {
                    return CloseDecision::KeepOpen;
                }
                let dirty = files.peek().is_some() && !buffers.can_leave(files.peek().as_ref());
                if !dirty && !preferences.peek().config.writer.confirm_exit {
                    return CloseDecision::Close;
                }
                previous_focus.set(Some(*platform.focused_accessibility_id.peek()));
                failure.set(preferences.peek().error.clone());
                pending.set(Some(if dirty { Prompt::Unsaved } else { Prompt::Exit }));
                CloseDecision::KeepOpen
            }));
        });
    });
    use_drop(|| {
        CLOSE.with(|handler| {
            handler.borrow_mut().take();
        });
    });
    let Some(prompt) = *pending.read() else {
        return rect().into_element();
    };
    let mut content = rect()
        .spacing(12.)
        .on_global_key_down(move |event: Event<KeyboardEventData>| {
            if event.key == Key::Named(NamedKey::Tab) {
                event.stop_propagation();
                event.prevent_default();
                let current = *Platform::get().focused_accessibility_id.peek();
                let index = actions.iter().position(|id| *id == current).unwrap_or(0);
                let step = if event.modifiers.contains(Modifiers::SHIFT) {
                    2
                } else {
                    1
                };
                actions[(index + step) % 3].request_focus();
            }
        })
        .child(PopupTitle::new(
            if prompt == Prompt::Unsaved {
                "There are unsaved changes."
            } else {
                "Close Recite?"
            }
            .to_owned(),
        ))
        .child(label().text(failure.read().clone().unwrap_or_else(|| {
            if prompt == Prompt::Unsaved {
                "Save your changes or keep a recovery copy for the next session.".into()
            } else if files.peek().is_none() {
                "Temporary example edits will be discarded when this window closes.".into()
            } else {
                "Your project is saved.".into()
            }
        })))
        .child(action_button(actions[0], "Keep editing", cancel));
    if prompt == Prompt::Unsaved {
        content = content
            .child(action_button(
                actions[1],
                "Keep recovery and close",
                move || {
                    buffers.harvest();
                    let state = buffers.model.peek();
                    if let (Ok(workbench), Some(project)) = (state.as_ref(), files.write().as_mut())
                    {
                        match project.checkpoint(workbench) {
                            Ok(()) => close_window(),
                            Err(error) => failure.set(Some(error.to_string())),
                        }
                    }
                },
            ))
            .child(action_button(
                actions[2],
                "Save and close",
                move || match buffers.save(files) {
                    Ok(()) => close_window(),
                    Err(error) => failure.set(Some(error)),
                },
            ));
    } else {
        content = content
            .child(
                rect()
                    .a11y_id(actions[1])
                    .a11y_focusable(true)
                    .a11y_role(AccessibilityRole::CheckBox)
                    .a11y_alt("Don't ask again when closing Recite")
                    .a11y_builder(|node| {
                        node.set_toggled(if *dont_ask.read() {
                            accesskit::Toggled::True
                        } else {
                            accesskit::Toggled::False
                        })
                    })
                    .cursor(CursorIcon::Pointer)
                    .padding(8.)
                    .border(
                        Border::new()
                            .width(if actions[1].is_focused() { 2. } else { 0. })
                            .fill((120, 140, 110)),
                    )
                    .on_all_press(move |_: Event<PressEventData>| {
                        let next = !*dont_ask.peek();
                        dont_ask.set(next);
                        actions[1].request_focus();
                    })
                    .child(label().text(if *dont_ask.read() {
                        "☑ Don't ask again when closing Recite"
                    } else {
                        "☐ Don't ask again when closing Recite"
                    })),
            )
            .child(action_button(actions[2], "Close Recite", move || {
                if *dont_ask.peek()
                    && let Err(error) = preferences
                        .write()
                        .update(recite_config::UserConfigEdit::WriterConfirmExit(false))
                {
                    failure.set(Some(error));
                    return;
                }
                close_window();
            }));
    }
    Popup::new()
        .on_close_request(move |_| cancel())
        .child(content)
        .into_element()
}

fn action_button(
    id: AccessibilityId,
    caption: &'static str,
    action: impl FnMut() + 'static,
) -> Element {
    let mut action = action;
    let action = EventHandler::new(move |()| action());
    rect().a11y_id(id).a11y_focusable(true).a11y_role(AccessibilityRole::Button).a11y_alt(caption)
        .cursor(CursorIcon::Pointer)
        .border(Border::new().width(if id.is_focused() { 2. } else { 1. }).fill((128, 128, 128))).padding(10.)
        .on_all_press(move |event: Event<PressEventData>| {
            if matches!(event.data(), PressEventData::Mouse(data) if data.button != Some(MouseButton::Left)) { return; }
            id.request_focus(); action.call(());
        }).child(label().text(caption)).into_element()
}
