use super::argument::ArgumentField;
use super::tree::{at, remove};
use crate::design::Options;
use crate::messages::{MsgId, text};
use crate::{
    design::{Button, tokens as t},
    editing::Writer,
};
use freya::prelude::*;
use recite_writer_model::RuleExpression;

#[derive(Clone)]
pub(super) struct ConditionControl {
    pub writer: Writer,
    pub node: RuleExpression,
    pub path: Vec<usize>,
    pub depth: usize,
}
impl PartialEq for ConditionControl {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node
            && self.path == other.path
            && self.depth == other.depth
            && self.writer.dark == other.writer.dark
    }
}
impl Component for ConditionControl {
    fn render(&self) -> impl IntoElement {
        let writer = self.writer;
        let node = &self.node;
        let path = self.path.clone();
        let depth = self.depth;
        let colors = t::colors();
        let mut actions = use_state(|| false);
        let mut body = rect()
            .width(Size::fill())
            .spacing(t::SPACE_SM)
            .padding(t::SPACE_SM)
            .corner_radius(t::RADIUS)
            .background(if depth.is_multiple_of(2) {
                Color::TRANSPARENT
            } else {
                colors.inset
            });
        match node {
            RuleExpression::Call {
                function,
                arguments,
            } => {
                let negate = path.clone();
                let remove_path = path.clone();
                let group_path = path.clone();
                body = body.child(
                    rect()
                        .horizontal()
                        .width(Size::fill())
                        .content(Content::Flex)
                        .child(
                            label()
                                .text(crate::palette::display_name(function))
                                .width(Size::flex(1.)),
                        )
                        .child(
                            Button::new()
                                .flat()
                                .expanded(*actions.read())
                                .named(format!("{}: {function}", text(MsgId::WriterRulesActions)))
                                .on_press(move |_| {
                                    let next = !*actions.peek();
                                    actions.set(next);
                                })
                                .child(text(MsgId::WriterRulesActions)),
                        ),
                );
                body = body.maybe_child((*actions.read()).then(|| {
                    rect()
                        .horizontal()
                        .spacing(t::SPACE_XS)
                        .child(
                            Button::new()
                                .flat()
                                .on_press(move |_| {
                                    super::change(writer, |rules| {
                                        if let Some(root) = &mut rules.condition
                                            && let Some(node) = at(root, &negate)
                                        {
                                            *node = RuleExpression::Not(Box::new(node.clone()));
                                        }
                                    })
                                })
                                .child(text(MsgId::WriterRulesNegate)),
                        )
                        .child(
                            Button::new()
                                .flat()
                                .on_press(move |_| {
                                    super::change(writer, |rules| {
                                        if let Some(root) = &mut rules.condition
                                            && let Some(node) = at(root, &group_path)
                                        {
                                            *node = RuleExpression::All(vec![node.clone()]);
                                        }
                                    });
                                })
                                .child(text(MsgId::WriterRulesGroup)),
                        )
                        .child(
                            Button::new()
                                .flat()
                                .named(format!("Remove condition {function}"))
                                .on_press(move |_| {
                                    super::change(writer, |rules| {
                                        rules.condition = rules
                                            .condition
                                            .take()
                                            .and_then(|node| remove(node, &remove_path));
                                    });
                                })
                                .child(text(MsgId::WriterRulesRemove)),
                        )
                }));
                for (index, argument) in arguments.iter().enumerate() {
                    let path = path.clone();
                    body = body.child(ArgumentField {
                        vim: writer.preferences.read().config.ui.keymap
                            == recite_config::Keymap::Vim,
                        argument: argument.clone(),
                        owner: function.clone(),
                        change: EventHandler::new(move |value: String| {
                            super::change(writer, |rules| {
                                if let Some(root) = &mut rules.condition
                                    && let Some(RuleExpression::Call { arguments, .. }) =
                                        at(root, &path)
                                    && let Some(argument) = arguments.get_mut(index)
                                {
                                    argument.value = value;
                                }
                            })
                        }),
                    });
                }
            }
            RuleExpression::All(items) | RuleExpression::Any(items) => {
                let group_path = path.clone();
                body = body.child(rect().width(Size::fill()).max_width(Size::px(600.)).child(
                    MatchMode {
                        writer,
                        path: group_path,
                        all: matches!(node, RuleExpression::All(_)),
                    },
                ));
                for (index, child) in items.iter().enumerate() {
                    let mut path = path.clone();
                    path.push(index);
                    body = body.child(ConditionControl {
                        writer,
                        node: child.clone(),
                        path,
                        depth: depth + 1,
                    });
                }
                if writer
                    .buffers
                    .rules
                    .read()
                    .as_ref()
                    .is_some_and(|r| !r.available_conditions.is_empty())
                {
                    body = body.child(super::add::AddRule {
                        writer,
                        effect: false,
                        path: Some(path),
                    });
                }
            }
            RuleExpression::Not(inner) => {
                let negate = path.clone();
                body = body.child(
                    Button::new()
                        .flat()
                        .on_press(move |_| {
                            super::change(writer, |rules| {
                                if let Some(root) = &mut rules.condition
                                    && let Some(node) = at(root, &negate)
                                    && let RuleExpression::Not(inner) = node
                                {
                                    *node = RuleExpression::Group(inner.clone());
                                }
                            })
                        })
                        .child(text(MsgId::WriterRulesRemoveNegation)),
                );
                let mut next = path;
                next.push(0);
                body = body.child(ConditionControl {
                    writer,
                    node: *inner.clone(),
                    path: next,
                    depth: depth + 1,
                });
            }
            RuleExpression::Group(inner) => {
                let mut next = path;
                next.push(0);
                return ConditionControl {
                    writer,
                    node: *inner.clone(),
                    path: next,
                    depth,
                }
                .into_element();
            }
        }
        body.into_element()
    }
}

#[derive(Clone)]
struct MatchMode {
    writer: Writer,
    path: Vec<usize>,
    all: bool,
}
impl PartialEq for MatchMode {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.all == other.all
    }
}
impl Component for MatchMode {
    fn render(&self) -> impl IntoElement {
        let writer = self.writer;
        let path = self.path.clone();
        Options {
            name: text(MsgId::WriterRulesMatch),
            labels: [text(MsgId::WriterRulesAll), text(MsgId::WriterRulesAny)],
            ids: [use_a11y(), use_a11y()],
            selected: usize::from(!self.all),
            vim: writer.preferences.read().config.ui.keymap == recite_config::Keymap::Vim,
            change: EventHandler::new(move |index| {
                super::change(writer, |rules| {
                    if let Some(root) = &mut rules.condition
                        && let Some(node) = at(root, &path)
                        && let RuleExpression::All(items) | RuleExpression::Any(items) = node
                    {
                        let items = items.clone();
                        *node = if index == 0 {
                            RuleExpression::All(items)
                        } else {
                            RuleExpression::Any(items)
                        };
                    }
                })
            }),
        }
    }
}
