//! Inputs are staged locally; Restart captures them together with catalogue bytes.
use crate::{
    design::{Button, SubmitAction, tokens as t},
    editing::Writer,
    messages::{MsgId, text},
};
use freya::prelude::*;
use recite_core::InterpolationType;
use std::collections::BTreeMap;

#[derive(Clone)]
pub(super) struct Controls {
    pub writer: Writer,
}
impl PartialEq for Controls {
    fn eq(&self, other: &Self) -> bool {
        self.writer.dark == other.writer.dark
    }
}
impl Component for Controls {
    fn render(&self) -> impl IntoElement {
        let mut writer = self.writer;
        let mut locale = writer.trial.locale;
        let variant = writer.trial.variant;
        let mut include_drafts = writer.trial.include_drafts;
        let values = writer.trial.values;
        let mut open = use_state(|| false);
        let id = use_a11y();
        let draft_id = use_a11y();
        let restart = SubmitAction {
            id,
            caption: text(MsgId::WriterRestartPreview),
            enabled: true,
            action: EventHandler::new(move |()| {
                let setup = super::prepare::prepare(
                    writer,
                    &locale.peek(),
                    &variant.peek(),
                    *include_drafts.peek(),
                    &values.peek(),
                );
                match setup {
                    Ok(setup) => {
                        if writer
                            .try_navigate(|model| model.start_preview_with(setup))
                            .is_ok()
                        {
                            let revision = writer
                                .buffers
                                .model
                                .peek()
                                .as_ref()
                                .map(|m| m.document().revision())
                                .unwrap_or_default();
                            writer.trial.snapshot.set(Some(super::Snapshot::capture(
                                format!(
                                    "{} · {} · {} {revision}",
                                    if locale.peek().is_empty() {
                                        text(MsgId::WriterSourceOnly)
                                    } else {
                                        locale.peek().clone()
                                    },
                                    text(if *include_drafts.peek() {
                                        MsgId::WriterTrialDrafts
                                    } else {
                                        MsgId::WriterTrialSaved
                                    }),
                                    text(MsgId::WriterSourceRevision)
                                ),
                                !locale.peek().is_empty(),
                                writer.localisation.peek().catalogue.as_ref(),
                            )));
                        }
                    }
                    Err(error) => writer.message.error(error),
                }
            }),
        };
        let shortcut = restart.clone();
        let mut body = rect()
            .width(Size::fill())
            .spacing(t::SPACE_MD)
            .on_global_key_down(move |event: Event<KeyboardEventData>| {
                if !writer.localisation.peek().modal_open()
                    && !*writer.settings_open.peek()
                    && crate::design::keyboard::submit_key(&event)
                {
                    event.prevent_default();
                    event.stop_propagation();
                    shortcut.run();
                }
            })
            .child(
                crate::design::actions()
                    .main_align(Alignment::Start)
                    .child(restart.button())
                    .child(
                        Button::new()
                            .flat()
                            .on_press(move |_| {
                                let next = !*open.peek();
                                open.set(next);
                            })
                            .child(text(MsgId::WriterTrialInputs)),
                    ),
            );
        if *open.read() {
            body = body
                .child(label().text(text(MsgId::WriterTrialInputsHelp)))
                .child(label().text(text(MsgId::WriterTargetLanguage)))
                .child(crate::localisation::LanguagePicker {
                    locale,
                    vim: writer.preferences.read().config.ui.keymap == recite_config::Keymap::Vim,
                })
                .child(
                    Button::new()
                        .flat()
                        .selected(locale.read().is_empty())
                        .on_press(move |_| locale.set(String::new()))
                        .child(text(MsgId::WriterSourceOnly)),
                )
                .child(label().text(text(MsgId::WriterWording)))
                .child(
                    Input::new(variant)
                        .placeholder(text(MsgId::WriterDefaultWording))
                        .width(Size::fill())
                        .on_pre_key_down(crate::closing::text_input_key),
                )
                .child(crate::design::checkbox(
                    draft_id,
                    text(MsgId::WriterIncludeDrafts),
                    *include_drafts.read(),
                    move |_| {
                        let next = !*include_drafts.peek();
                        include_drafts.set(next);
                    },
                ));
            let bindings = writer
                .buffers
                .model
                .read()
                .as_ref()
                .map(|m| m.document().preview_bindings())
                .unwrap_or_default();
            for binding in bindings {
                let name = binding.value.trim_start_matches('$').to_owned();
                body = body.child(
                    rect()
                        .key(name.clone())
                        .width(Size::fill())
                        .child(ValueInput {
                            name,
                            values,
                            kind: binding.value_type,
                        }),
                );
            }
        }
        body
    }
}

#[derive(Clone, PartialEq)]
struct ValueInput {
    name: String,
    kind: InterpolationType,
    values: State<BTreeMap<String, String>>,
}
impl Component for ValueInput {
    fn render(&self) -> impl IntoElement {
        let name = self.name.clone();
        let mut values = self.values;
        let mut value = use_state(String::new);
        let id = use_a11y();
        value.set_if_modified(values.read().get(&name).cloned().unwrap_or_default());
        if self.kind == InterpolationType::Boolean {
            let checked = values
                .read()
                .get(&name)
                .is_some_and(|value| value == "true");
            return crate::design::checkbox(id, name.clone(), checked, move |_| {
                values.write().insert(name.clone(), (!checked).to_string());
            });
        }
        rect()
            .width(Size::fill())
            .spacing(t::SPACE_SM)
            .child(label().text(name.clone()))
            .child(
                Input::new(value)
                    .placeholder(name.clone())
                    .width(Size::fill())
                    .on_pre_key_down(crate::closing::text_input_key)
                    .on_validate(move |input: InputValidator| {
                        values.write().insert(name.clone(), input.text().clone());
                    }),
            )
            .into_element()
    }
}
