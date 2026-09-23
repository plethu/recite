//! Search is a transient dialog; cancelling returns to its invoker.
use super::{Command, SearchMode, targets};
use crate::commands::CommandExt;
use crate::{
    design::{Button, Dialog, SearchField, SubmitAction, tokens as t},
    editing::Writer,
    messages::{MsgId, text},
};
use freya::prelude::*;

#[derive(Clone)]
pub(crate) struct Palette {
    pub writer: Writer,
}
impl PartialEq for Palette {
    fn eq(&self, other: &Self) -> bool {
        self.writer.dark == other.writer.dark
    }
}
#[derive(Clone, PartialEq)]
enum Item {
    Command(Command),
    Target(targets::Target),
}
impl Item {
    fn label(&self, writer: Writer) -> String {
        match self {
            Self::Command(c) => c.label(writer),
            Self::Target(t) => t.label(),
        }
    }
    fn enabled(&self, writer: Writer) -> bool {
        match self {
            Self::Command(c) => c.enabled(writer),
            Self::Target(_) => true,
        }
    }
    fn run(&self, mut writer: Writer) {
        writer.command_search.close();
        match self {
            Self::Command(Command::Settings) => {
                crate::settings::open(writer, *writer.command_search.return_focus.peek());
            }
            Self::Command(c) => c.run(writer),
            Self::Target(t) => {
                if let Err(error) = t.open(writer) {
                    writer.message.error(error);
                }
            }
        }
    }
}
impl Component for Palette {
    fn render(&self) -> impl IntoElement {
        let writer = self.writer;
        let mut query = use_state(String::new);
        let active = use_state(|| None::<usize>);
        let input = use_a11y();
        let close_id = use_a11y();
        let mut previous = use_state(|| None);
        let mut scroll = use_scroll_controller(ScrollConfig::default);
        let mode = *writer.command_search.mode.read();
        use_after_side_effect(move || {
            let mode = *writer.command_search.mode.read();
            if mode != *previous.peek() {
                previous.set(mode);
                query.set(String::new());
                if mode.is_some() {
                    scroll.scroll_to_y(0);
                    input.request_focus();
                }
            }
        });
        let all = use_memo(move || match *writer.command_search.mode.read() {
            Some(SearchMode::Commands) => Command::ALL
                .iter()
                .copied()
                .map(Item::Command)
                .collect::<Vec<_>>(),
            Some(SearchMode::GoTo) => targets::all(writer).into_iter().map(Item::Target).collect(),
            None => Vec::new(),
        });
        let items = use_memo(move || {
            let mut matches: Vec<_> = all
                .read()
                .iter()
                .cloned()
                .filter_map(|item| {
                    let query = query.read();
                    if *writer.command_search.mode.read() == Some(SearchMode::Commands)
                        && writer.preferences.read().config.ui.keymap == recite_config::Keymap::Vim
                        && let Some(command) = super::vim::alias(&query)
                    {
                        return (item == Item::Command(command)).then_some((0, item));
                    }
                    targets::rank(&item.label(writer), &query).map(|rank| (rank, item))
                })
                .collect();
            matches.sort_by_key(|(rank, _)| *rank);
            matches
                .into_iter()
                .map(|(_, item)| item)
                .collect::<Vec<_>>()
        });
        crate::design::use_list_reveal(Some(query), active, scroll, t::picker_row_height(), 3);
        let Some(mode) = mode else {
            return rect().into_element();
        };
        let mut content = rect()
            .width(Size::fill())
            .spacing(t::SPACE_SM)
            .child(SearchField {
                insert_request: None,
                query,
                id: input,
                placeholder: text(if mode == SearchMode::Commands {
                    MsgId::WriterWorkspaceCommandSearch
                } else {
                    MsgId::WriterWorkspaceGoToSearch
                }),
                active,
                count: items.read().len(),
                vim: false,
                changed: EventHandler::new(|()| {}),
                activate: EventHandler::new(move |index: usize| {
                    if let Some(item) = items.peek().get(index).cloned()
                        && item.enabled(writer)
                    {
                        item.run(writer);
                    }
                }),
            });
        if mode == SearchMode::Commands
            && writer.preferences.read().config.ui.keymap == recite_config::Keymap::Vim
        {
            content = content.child(
                label()
                    .text("w Save · wa Save all · q Close")
                    .font_size(t::small()),
            );
        }
        let rows = items.read().clone();
        if rows.is_empty() {
            content = content.child(label().text(text(MsgId::WriterWorkspaceNoCommandResults)));
        }
        content = content.child(
            VirtualScrollView::new_with_data(
                (rows.clone(), *active.read()),
                move |entry, (rows, active)| {
                    let item = rows[entry.index].clone();
                    let title = item.label(writer);
                    let enabled = item.enabled(writer);
                    let detail = match &item {
                        Item::Command(c) if enabled => c.shortcut(writer),
                        _ if !enabled => text(MsgId::WriterWorkspaceCommandUnavailable),
                        _ => String::new(),
                    };
                    rect()
                        .key(entry.index)
                        .height(Size::px(entry.size))
                        .width(Size::fill())
                        .child(
                            Button::new()
                                .flat()
                                .width(Size::fill())
                                .named(title.clone())
                                .shortcut(match &item {
                                    Item::Command(c) => c.shortcut(writer),
                                    Item::Target(_) => String::new(),
                                })
                                .enabled(enabled)
                                .selected(*active == Some(entry.index))
                                .on_press(move |_| item.run(writer))
                                .child(
                                    rect()
                                        .horizontal()
                                        .width(Size::fill())
                                        .content(Content::Flex)
                                        .child(label().text(title).width(Size::flex(1.)))
                                        .child(label().text(detail).font_size(t::small())),
                                ),
                        )
                        .into_element()
                },
            )
            .length(rows.len())
            .item_size(t::picker_row_height())
            .height(Size::px(320.))
            .scroll_controller(scroll),
        );
        Dialog {
            title: text(if mode == SearchMode::Commands {
                MsgId::WriterWorkspaceCommands
            } else {
                MsgId::WriterWorkspaceGoTo
            }),
            content: content.into_element(),
            actions: rect().into_element(),
            primary: SubmitAction {
                id: close_id,
                caption: text(MsgId::WriterWorkspaceCloseCommands),
                enabled: true,
                action: EventHandler::new(move |()| writer.command_search.close()),
            },
            focus_order: vec![input, close_id],
            close: EventHandler::new(move |()| writer.command_search.close()),
            reduced_motion: writer.preferences.read().config.writer.reduced_motion,
            dismissal_only: true,
        }
        .into_element()
    }
}
