//! Reply rules edit the recoverable source draft and apply through the authoring model.
use crate::design::tokens::ProseTypography;
use crate::messages::{MsgId, text};
mod add;
mod argument;
mod condition;
mod effect;
mod tree;
use crate::{
    design::{Button, SubmitAction, tokens as t},
    editing::{Pane, Writer, editor_data},
};
use freya::prelude::*;
use recite_writer_model::{ReplyRules, View};

pub(super) fn open(writer: Writer, passage: &str) -> Result<(), String> {
    let location = crate::navigation::location::Location {
        screen: crate::navigation::location::Screen::Rules,
        passage: Some(passage.to_owned()),
        source: false,
        beat: None,
        ..crate::navigation::snapshot(writer)
    };
    crate::navigation::route(
        writer,
        location,
        None,
        EventHandler::new(move |result: Result<(), String>| {
            if let Err(error) = result {
                let mut message = writer.message;
                message.error(error);
            }
        }),
    )
}

fn change(mut writer: Writer, edit: impl FnOnce(&mut ReplyRules)) {
    if writer.work.busy() {
        return;
    }
    let result = (|| {
        let mut state = writer.buffers.rules.write();
        let rules = state.as_mut().ok_or("Open a reply first.")?;
        if rules.source().map_err(|e| e.to_string())? != writer.buffers.editor.peek().rope {
            return Err(
                "The source draft changed. Reopen reply rules after applying or discarding it."
                    .to_owned(),
            );
        }
        let mut next = rules.clone();
        edit(&mut next);
        let source = next.source().map_err(|e| e.to_string())?;
        writer
            .buffers
            .editor
            .set(editor_data(&source, true, writer.dark));
        *rules = next;
        Ok(())
    })();
    writer.buffers.harvest();
    if let Err(error) = result {
        writer.message.error(error);
    }
}

fn reload(mut writer: Writer, id: &str) {
    if !writer.work.busy() {
        writer.buffers.rules.set(None);
        let _ = crate::editor_work::rules(writer, id);
    }
}

#[derive(Clone)]
pub(super) struct RulesScreen {
    pub writer: Writer,
}
impl PartialEq for RulesScreen {
    fn eq(&self, other: &Self) -> bool {
        self.writer.dark == other.writer.dark
    }
}
impl Component for RulesScreen {
    fn render(&self) -> impl IntoElement {
        let mut writer = self.writer;
        let apply_id = use_a11y();
        let colors = t::colors();
        use_side_effect(move || {
            let stale = writer.buffers.model.read().as_ref().ok().and_then(|model| {
                writer
                    .buffers
                    .rules
                    .read()
                    .as_ref()
                    .filter(|rules| !model.has_draft() && !rules.belongs_to(model.document()))
                    .map(|rules| rules.passage.clone())
            });
            if let Some(passage) = stale {
                reload(writer, &passage);
            }
        });
        if writer.work.busy() {
            let caption = writer.work.caption().unwrap_or_default();
            return rect()
                .width(Size::fill())
                .padding(t::SPACE_MD)
                .a11y_role(AccessibilityRole::Status)
                .a11y_alt(caption.clone())
                .child(label().text(caption))
                .into_element();
        }
        let state = writer.buffers.rules.read();
        let Some(rules) = state.as_ref() else {
            return label()
                .text(if writer.work.busy() {
                    "Preparing reply rules…".into()
                } else {
                    text(MsgId::WriterRulesOpen)
                })
                .into_element();
        };
        let rules = rules.clone();
        let validation = writer
            .buffers
            .model
            .read()
            .as_ref()
            .map_err(|error| Some(error.to_string()))
            .and_then(|m| {
                rules
                    .validate_draft(m.document())
                    .map_err(|error| match error {
                        // Value errors render beside their owning input.
                        recite_writer_model::EditError::InvalidRuleValue { .. } => None,
                        other => Some(other.to_string()),
                    })
            });
        let consistent = validation
            .as_ref()
            .is_ok_and(|source| writer.buffers.editor.read().rope == source.as_str());
        let enabled = rules.changed() && validation.is_ok() && consistent && !writer.work.busy();
        let primary = SubmitAction {
            id: apply_id,
            caption: text(MsgId::WriterRulesApply),
            enabled,
            action: EventHandler::new(move |()| {
                if let Err(error) =
                    crate::editor_work::edit(writer, recite_writer_model::EditOperation::Apply)
                {
                    writer.message.error(error);
                }
            }),
        };
        let shortcut = primary.clone();
        let discard_id = rules.passage.clone();
        let return_id = rules.passage.clone();
        let mut body = rect()
            .width(Size::fill())
            .max_width(Size::px(940.))
            .padding(t::SPACE_XL)
            .spacing(t::SPACE_MD)
            .child(
                crate::design::actions()
                    .main_align(Alignment::Start)
                    .child(
                        Button::new()
                            .flat()
                            .on_press(move |_| {
                                if writer
                                    .try_navigate(|m| m.select(View::Passage(return_id.clone())))
                                    .is_ok()
                                {
                                    writer.pane.set(Pane::Script);
                                }
                            })
                            .child(text(MsgId::WriterRulesReturn)),
                    )
                    .child(
                        Button::new()
                            .flat()
                            .on_press(move |_| writer.pane.set(Pane::Map))
                            .child(text(MsgId::WriterRulesSource)),
                    ),
            )
            .child(
                label()
                    .text(text(MsgId::WriterRulesTitle))
                    .font_size(t::heading()),
            )
            .child(
                rect()
                    .prose_font()
                    .child(label().text(rules.text.clone()).font_size(t::heading())),
            )
            .child(label().text(text(MsgId::WriterRulesAvailability)));
        body = match &rules.condition {
            Some(condition) => body.child(condition::ConditionControl {
                writer,
                node: condition.clone(),
                path: Vec::new(),
                depth: 0,
            }),
            None => body.child(
                label()
                    .text(text(MsgId::WriterRulesAlways))
                    .color(colors.muted),
            ),
        };
        if rules.condition.is_none() && !rules.available_conditions.is_empty() {
            body = body.child(add::AddRule {
                writer,
                effect: false,
                path: None,
            });
        }
        if rules.condition.is_some() {
            body = body.child(
                Button::new()
                    .flat()
                    .on_press(move |_| change(writer, |r| r.condition = None))
                    .child(text(MsgId::WriterRulesMakeAlways)),
            );
        }
        body = body.child(label().text(text(MsgId::WriterRulesChosen)));
        if let Some(destination) = &rules.destination {
            body = body.child(
                label()
                    .text(format!(
                        "{} · {}",
                        destination,
                        text(MsgId::WriterRulesDestination)
                    ))
                    .color(colors.muted),
            );
        }
        if rules.effects.is_empty() {
            body = body.child(
                label()
                    .text(text(MsgId::WriterRulesNoEffects))
                    .color(colors.muted),
            );
        }
        for (index, effect) in rules.effects.iter().enumerate() {
            body = body.child(effect::EffectControl {
                writer,
                effect: effect.clone(),
                index,
                count: rules.effects.len(),
                reorder: rules.effect_order_editable,
            });
        }
        if !rules.available_effects.is_empty() && rules.effect_order_editable {
            body = body.child(add::AddRule {
                writer,
                effect: true,
                path: None,
            });
        }
        if rules.has_other_statements {
            body = body.child(
                label()
                    .text(text(MsgId::WriterRulesPreserved))
                    .color(colors.muted),
            );
        }
        if !consistent {
            body = body.child(
                label()
                    .text(text(MsgId::WriterRulesSourceChanged))
                    .color(colors.muted),
            );
        } else if rules.changed()
            && let Err(Some(error)) = validation
        {
            body = body.child(label().text(error).color(colors.muted));
        }
        body = body.child(
            crate::design::actions()
                .main_align(Alignment::Start)
                .child(primary.button())
                .child(
                    Button::new()
                        .enabled(rules.changed() && !writer.work.busy())
                        .on_press(move |_| {
                            writer.perform(|m| {
                                m.discard();
                                Ok(())
                            });
                            reload(writer, &discard_id);
                        })
                        .child(text(MsgId::WriterRulesDiscard)),
                ),
        );
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .child(
                ScrollView::new()
                    .width(Size::fill())
                    .height(Size::fill())
                    .child(body),
            )
            .on_global_key_down(move |event: Event<KeyboardEventData>| {
                if !*writer.settings_open.peek()
                    && !writer.localisation.peek().modal_open()
                    && crate::design::keyboard::submit_key(&event)
                {
                    event.prevent_default();
                    event.stop_propagation();
                    shortcut.run();
                }
            })
            .into_element()
    }
}
