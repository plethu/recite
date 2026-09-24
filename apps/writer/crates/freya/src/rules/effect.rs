use super::argument::ArgumentField;
use crate::messages::{MsgId, text};
use crate::{
    design::{Button, tokens as t},
    editing::Writer,
};
use freya::prelude::*;
use recite_core::ast::EffectMode;
use recite_writer_model::RuleEffect;
#[derive(Clone)]
pub(super) struct EffectControl {
    pub writer: Writer,
    pub effect: RuleEffect,
    pub index: usize,
    pub count: usize,
    pub reorder: bool,
}
impl PartialEq for EffectControl {
    fn eq(&self, other: &Self) -> bool {
        self.effect == other.effect
            && self.index == other.index
            && self.count == other.count
            && self.reorder == other.reorder
            && self.writer.dark == other.writer.dark
    }
}
impl Component for EffectControl {
    fn render(&self) -> impl IntoElement {
        let Self {
            writer,
            ref effect,
            index,
            count,
            reorder,
        } = *self;
        let mut expanded = use_state(|| false);
        let colors = t::colors();
        let mut body = rect()
            .width(Size::fill())
            .spacing(t::SPACE_MD)
            .padding(t::SPACE_MD)
            .corner_radius(t::RADIUS)
            .background(colors.surface)
            .child(
                rect()
                    .horizontal()
                    .width(Size::fill())
                    .content(Content::Flex)
                    .child(
                        Button::new()
                            .flat()
                            .width(Size::flex(1.))
                            .named(format!("Edit effect {}", index + 1))
                            .on_press(move |_| {
                                let next = !*expanded.peek();
                                expanded.set(next);
                            })
                            .expanded(*expanded.read())
                            .child(label().width(Size::fill()).text(format!(
                                "{} {}. {} · {}",
                                if *expanded.read() { "▾" } else { "▸" },
                                index + 1,
                                crate::palette::display_name(&effect.function),
                                mode(effect.mode)
                            ))),
                    )
                    .child(
                        Button::new()
                            .flat()
                            .named(format!("Remove effect {}", index + 1))
                            .enabled(reorder)
                            .on_press(move |_| {
                                super::change(writer, |r| {
                                    r.effects.remove(index);
                                })
                            })
                            .child(text(MsgId::WriterRulesRemove)),
                    )
                    .child(
                        Button::new()
                            .flat()
                            .named(format!("Move effect {} up", index + 1))
                            .enabled(reorder && index > 0)
                            .on_press(move |_| {
                                super::change(writer, |r| r.effects.swap(index, index - 1))
                            })
                            .child("↑"),
                    )
                    .child(
                        Button::new()
                            .flat()
                            .named(format!("Move effect {} down", index + 1))
                            .enabled(reorder && index + 1 < count)
                            .on_press(move |_| {
                                super::change(writer, |r| r.effects.swap(index, index + 1))
                            })
                            .child("↓"),
                    ),
            );
        if *expanded.read() {
            let mut modes = crate::design::actions()
                .main_align(Alignment::Start)
                .child(label().text(text(MsgId::WriterRulesDelivery)));
            for value in &effect.allowed_modes {
                let value = *value;
                modes = modes.child(
                    Button::new()
                        .flat()
                        .selected(value == effect.mode)
                        .on_press(move |_| super::change(writer, |r| r.effects[index].mode = value))
                        .child(mode(value)),
                );
            }
            body = body.child(modes);
            for (arg, argument) in effect.arguments.iter().enumerate() {
                body = body.child(ArgumentField {
                    vim: writer.preferences.read().config.ui.keymap == recite_config::Keymap::Vim,
                    argument: argument.clone(),
                    owner: effect.function.clone(),
                    change: EventHandler::new(move |value: String| {
                        super::change(writer, |r| r.effects[index].arguments[arg].value = value)
                    }),
                });
            }
        } else if let Some(error) = effect
            .arguments
            .iter()
            .find_map(|argument| argument.validate().err())
        {
            body = body.child(
                label()
                    .text(format!(
                        "{}: {error}",
                        crate::palette::display_name(&effect.function)
                    ))
                    .font_size(t::small())
                    .color(colors.error),
            );
        }
        body.into_element()
    }
}
fn mode(mode: EffectMode) -> String {
    match mode {
        EffectMode::Immediate => text(MsgId::WriterRulesImmediate),
        EffectMode::Blocking => text(MsgId::WriterRulesBlocking),
        EffectMode::Deferred => text(MsgId::WriterRulesDeferred),
    }
}
