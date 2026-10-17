//! "New project…": a folder and a name become a starter project, which then opens.
use crate::design::{Button, Dialog, SubmitAction, tokens as t};
use freya::prelude::*;

#[derive(Clone, PartialEq)]
pub(crate) struct NewProjectDialog {
    pub open: State<bool>,
    /// Receives the new project's folder once it has been written.
    pub created: EventHandler<String>,
    pub return_focus: AccessibilityId,
    pub reduced_motion: bool,
}

impl Component for NewProjectDialog {
    fn render(&self) -> impl IntoElement {
        let mut open = self.open;
        let name = use_state(String::new);
        let folder = use_state(String::new);
        let mut error = use_state(|| None::<String>);
        let name_id = use_a11y();
        let folder_id = use_a11y();
        let browse_id = use_a11y();
        let cancel_id = use_a11y();
        let create_id = use_a11y();
        use_hook(move || name_id.request_focus());
        let return_focus = self.return_focus;
        let close = EventHandler::new(move |()| {
            open.set(false);
            return_focus.request_focus();
        });
        let created = self.created.clone();
        let create = EventHandler::new(move |()| {
            let target = folder.peek().trim().to_owned();
            let title = name.peek().trim().to_owned();
            match recite_config::create_starter_project(std::path::Path::new(&target), &title) {
                Ok(_) => {
                    error.set(None);
                    open.set(false);
                    created.call(target);
                }
                Err(failure) => error.set(Some(failure.to_string())),
            }
        });
        let ready = !name.read().trim().is_empty() && !folder.read().trim().is_empty();
        let colors = t::colors();
        let content = rect()
            .width(Size::fill())
            .spacing(t::SPACE_SM)
            .child(label().text("Project name"))
            .child(
                Input::new(name)
                    .a11y_id(name_id)
                    .width(Size::fill())
                    .placeholder("Night Bus"),
            )
            .child(label().text("Folder"))
            .child(crate::design::PathField {
                value: folder,
                id: folder_id,
                browse_id,
                kind: crate::design::PathKind::NewProject,
                enabled: true,
                submit: create.clone(),
            })
            .child(t::meta(
                "Writes recite.project.toml and one scene with a default beat. The folder is created if needed.",
            ))
            .maybe_child(error.read().clone().map(|message| {
                label()
                    .text(message)
                    .color(colors.error)
                    .a11y_role(AccessibilityRole::Alert)
            }));
        let cancel = close.clone();
        Dialog {
            title: "New project".into(),
            content: content.into_element(),
            actions: Button::new()
                .a11y_id(cancel_id)
                .on_press(move |_| cancel.call(()))
                .child("Cancel")
                .into_element(),
            primary: SubmitAction {
                id: create_id,
                caption: "Create".into(),
                enabled: ready,
                action: create,
            },
            focus_order: vec![name_id, folder_id, browse_id, cancel_id, create_id],
            close,
            reduced_motion: self.reduced_motion,
            dismissal_only: false,
        }
    }
}
