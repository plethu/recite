//! Pending source edits and jobs are actionable at the close boundary.
use crate::{
    design::{Button, Dialog, SubmitAction, tokens as t},
    editing::Writer,
};
use freya::prelude::*;
#[derive(Clone)]
pub(super) struct Blockers {
    pub writer: Writer,
    pub cancel: EventHandler<()>,
    pub retry: EventHandler<()>,
}
impl PartialEq for Blockers {
    fn eq(&self, other: &Self) -> bool {
        self.writer.dark == other.writer.dark
            && self.cancel == other.cancel
            && self.retry == other.retry
    }
}
impl Component for Blockers {
    fn render(&self) -> impl IntoElement {
        let mut writer = self.writer;
        let mut failure = use_state(|| None::<String>);
        let ids = [
            use_a11y(),
            use_a11y(),
            use_a11y(),
            use_a11y(),
            use_a11y(),
            use_a11y(),
        ];
        use_after_side_effect(move || ids[0].request_focus());
        let files = writer.files.read();
        let Some(project) = files.as_ref() else {
            return rect().into_element();
        };
        let builds = project.builds.busy();
        let generating = project.declarations.as_ref().is_some_and(|s| s.busy());
        let dirty = project.declarations.as_ref().is_some_and(|s| s.dirty());
        let path = project
            .declarations
            .as_ref()
            .and_then(|s| s.source.as_ref())
            .map(|s| s.path.display().to_string());
        let mut content = rect().width(Size::fill()).spacing(t::SPACE_SM);
        let mut order = vec![ids[0]];
        if let Some(error) = failure.read().as_ref() {
            content = content.child(label().text(error.clone()));
        }
        if builds || generating {
            content = content.child(label().text(crate::messages::text(
                crate::messages::MsgId::WriterGuiJobsPending,
            )));
            order.push(ids[1]);
            content = content.child(
                Button::new()
                    .a11y_id(ids[1])
                    .child(crate::messages::text(
                        crate::messages::MsgId::WriterGuiCancelRunningJobs,
                    ))
                    .on_press(move |_| {
                        if let Some(project) = writer.files.peek().as_ref() {
                            project.builds.cancel();
                            if let Some(session) = &project.declarations {
                                session.cancel();
                            }
                        }
                    }),
            );
        }
        if dirty {
            content = content.child(label().text(format!(
                "Unsaved declaration source: {}",
                path.unwrap_or_default()
            )));
            for (index, caption, save) in [
                (2, "Save and regenerate declarations", true),
                (3, "Discard declaration draft", false),
            ] {
                if !generating {
                    order.push(ids[index]);
                }
                content = content.child(
                    Button::new()
                        .a11y_id(ids[index])
                        .enabled(!generating)
                        .child(caption)
                        .on_press(move |_| {
                            let result = writer
                                .files
                                .write()
                                .as_mut()
                                .and_then(|p| p.declarations.as_mut())
                                .map(|s| {
                                    if save {
                                        s.save_and_generate()
                                    } else {
                                        s.discard()
                                    }
                                });
                            if let Some(Err(error)) = result {
                                failure.set(Some(error.to_string()));
                            }
                        }),
                );
            }
        }
        let cancel = self.cancel.clone();
        let view = self.cancel.clone();
        order.push(ids[4]);
        order.push(ids[5]);
        Dialog {
            title: "Before closing Recite".into(),
            content: content.into_element(),
            actions: crate::design::actions()
                .child(
                    Button::new()
                        .a11y_id(ids[0])
                        .child(crate::messages::text(
                            crate::messages::MsgId::WriterGuiKeepEditing,
                        ))
                        .on_press(move |_| cancel.call(())),
                )
                .child(
                    Button::new()
                        .a11y_id(ids[4])
                        .child(if dirty || generating {
                            "Open declarations"
                        } else {
                            "Open build"
                        })
                        .on_press(move |_| {
                            view.call(());
                            if dirty || generating {
                                crate::declarations::open(writer);
                            } else {
                                crate::builds::open(writer);
                            }
                        }),
                )
                .into_element(),
            primary: SubmitAction {
                id: ids[5],
                caption: "Retry closing".into(),
                enabled: !builds && !generating && !dirty,
                action: self.retry.clone(),
            },
            close: self.cancel.clone(),
            focus_order: order,
            dismissal_only: false,
            reduced_motion: writer.preferences.read().config.writer.reduced_motion,
        }
        .into_element()
    }
}
