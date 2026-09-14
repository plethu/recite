//! Bridge the single writer window's native close request to its document session.
use std::cell::RefCell;

use freya::prelude::*;

use crate::{files::Buffers, project::ProjectFiles};

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

pub(super) fn controls(buffers: Buffers, mut files: State<Option<ProjectFiles>>) -> Element {
    let mut pending = use_state(|| false);
    let mut failure = use_state(|| None::<String>);
    let mut previous_focus = use_state(|| None::<AccessibilityId>);
    let actions = [use_a11y(), use_a11y(), use_a11y()];
    let platform = Platform::get();
    let mut cancel = move || {
        pending.set(false);
        if let Some(id) = *previous_focus.peek() {
            id.request_focus();
        }
    };
    use_after_side_effect(move || {
        if *pending.read() {
            actions[0].request_focus();
        }
    });
    use_hook(move || {
        CLOSE.with(|handler| {
            *handler.borrow_mut() = Some(Box::new(move || {
                if files.peek().is_none() || buffers.can_leave(files.peek().as_ref()) {
                    return CloseDecision::Close;
                }
                if !*pending.peek() {
                    previous_focus.set(Some(*platform.focused_accessibility_id.peek()));
                    failure.set(None);
                    pending.set(true);
                }
                CloseDecision::KeepOpen
            }))
        });
    });
    use_drop(|| {
        CLOSE.with(|handler| {
            handler.borrow_mut().take();
        })
    });
    if !*pending.read() {
        return rect().into_element();
    }
    Popup::new()
        .on_close_request(move |_| cancel())
        .child(
            rect()
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
                        actions[(index + step) % actions.len()].request_focus();
                    }
                })
                .spacing(12.)
                .child(PopupTitle::new("There are unsaved changes.".to_owned()))
                .child(
                    label().text(
                        failure.read().clone().unwrap_or_else(|| {
                            "Your draft can be kept for the next session.".into()
                        }),
                    ),
                )
                .child(action_button(actions[0], "Keep editing", cancel))
                .child(action_button(
                    actions[1],
                    "Keep recovery and close",
                    move || {
                        buffers.harvest();
                        let state = buffers.model.peek();
                        let Ok(workbench) = state.as_ref() else {
                            return;
                        };
                        if let Some(project) = files.write().as_mut() {
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
                )),
        )
        .into_element()
}

// Stable focus identities let the dialog own its keyboard loop and restore the caller.
fn action_button(
    id: AccessibilityId,
    caption: &'static str,
    action: impl FnMut() + 'static,
) -> Element {
    let mut action = action;
    let action = EventHandler::new(move |()| action());
    rect()
        .a11y_id(id)
        .a11y_focusable(true)
        .a11y_role(AccessibilityRole::Button)
        .cursor(CursorIcon::Pointer)
        .border(
            Border::new()
                .width(if id.is_focused() { 3. } else { 1. })
                .fill((128, 128, 128)),
        )
        .padding(10.)
        .on_all_press(move |event: Event<PressEventData>| {
            let activate = match event.data() {
                PressEventData::Mouse(data) => data.button == Some(MouseButton::Left),
                PressEventData::Touch(_) | PressEventData::Keyboard(_) => true,
            };
            if activate {
                id.request_focus();
                action.call(());
            }
        })
        .child(label().text(caption))
        .into_element()
}
