//! One close policy for native window requests and application keyboard shortcuts.
use crate::design::tokens as t;
use crate::{buffers::Buffers, project::ProjectFiles};
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
    if crate::design::keyboard::submit_key(&event)
        || crate::editing::is_workspace_key(&event)
        || (event.modifiers == Modifiers::ALT
            && matches!(event.code, Code::ArrowLeft | Code::ArrowRight))
    {
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
    localisation: State<crate::localisation::Localisation>,
    mut message: crate::feedback::Feedback,
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
                if localisation.peek().dirty() {
                    message.error(crate::localisation::close_drafts_message());
                    return CloseDecision::KeepOpen;
                }
                if pending.peek().is_some() {
                    return CloseDecision::KeepOpen;
                }
                let mut dirty = files.peek().is_some() && !buffers.can_leave(files.peek().as_ref());
                let mut close_error = None;
                if !dirty && !preferences.peek().config.writer.confirm_exit {
                    match flush_recovery(buffers, files) {
                        Ok(()) => return CloseDecision::Close,
                        Err(error) => {
                            dirty = true;
                            close_error = Some(error);
                        }
                    }
                }
                previous_focus.set(Some(*platform.focused_accessibility_id.peek()));
                failure.set(close_error.or_else(|| preferences.peek().error.clone()));
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
    let mut content =
        rect()
            .spacing(t::SPACE_MD)
            .child(label().text(failure.read().clone().unwrap_or_else(|| {
                if prompt == Prompt::Unsaved {
                    "Save your changes or keep a recovery copy for the next session.".into()
                } else if files.peek().is_none() {
                    "Temporary example edits will be discarded when this window closes.".into()
                } else {
                    "Your project is saved.".into()
                }
            })));
    let mut footer =
        crate::design::actions().child(action_button(actions[0], "Keep editing", cancel));
    let primary;
    if prompt == Prompt::Unsaved {
        footer = footer.child(action_button(
            actions[1],
            "Keep recovery and close",
            move || {
                buffers.harvest();
                let state = buffers.model.peek();
                if let (Ok(workbench), Some(project)) = (state.as_ref(), files.write().as_mut()) {
                    match project.checkpoint(workbench) {
                        Ok(()) => close_window(),
                        Err(error) => failure.set(Some(error.to_string())),
                    }
                }
            },
        ));
        primary = crate::design::DialogAction {
            id: actions[2],
            caption: "Save and close".into(),
            enabled: true,
            action: EventHandler::new(move |()| match buffers.save(files) {
                Ok(()) => close_window(),
                Err(error) => failure.set(Some(error)),
            }),
        };
    } else {
        content = content.child(crate::design::checkbox(
            actions[1],
            "Don't ask again when closing Recite",
            *dont_ask.read(),
            move |_| {
                let next = !*dont_ask.peek();
                dont_ask.set(next);
            },
        ));
        primary = crate::design::DialogAction {
            id: actions[2],
            caption: "Close Recite".into(),
            enabled: true,
            action: EventHandler::new(move |()| {
                if *dont_ask.peek()
                    && let Err(error) = preferences
                        .write()
                        .update(recite_config::UserConfigEdit::WriterConfirmExit(false))
                {
                    failure.set(Some(error));
                    return;
                }
                match flush_recovery(buffers, files) {
                    Ok(()) => close_window(),
                    Err(error) => failure.set(Some(error)),
                }
            }),
        };
    }
    crate::design::Dialog {
        primary,
        title: if prompt == Prompt::Unsaved {
            "Unsaved changes".into()
        } else {
            "Close Recite?".into()
        },
        reduced_motion: preferences.read().config.writer.reduced_motion,
        close: EventHandler::new(move |()| cancel()),
        content: content.into_element(),
        focus_order: actions.to_vec(),
        actions: footer.into_element(),
    }
    .into_element()
}

fn action_button(
    id: AccessibilityId,
    caption: &'static str,
    action: impl FnMut() + 'static,
) -> crate::design::Button {
    let mut action = action;
    crate::design::Button::new()
        .a11y_id(id)
        .named(caption)
        .on_press(move |_| action())
        .child(label().text(caption))
}

// A clean source can still have a queued recovery deletion after undo or save.
// Closing must observe its durability result instead of relying on Drop.
fn flush_recovery(buffers: Buffers, mut files: State<Option<ProjectFiles>>) -> Result<(), String> {
    buffers.harvest();
    let model = buffers.model.peek();
    if let (Ok(workbench), Some(project)) = (model.as_ref(), files.write().as_mut()) {
        project
            .checkpoint(workbench)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}
