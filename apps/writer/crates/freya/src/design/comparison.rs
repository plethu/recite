//! Shared version presentation. Callers own identity, freshness and resolution.
mod diff;
mod source;
use super::{Button, tokens as t};
use crate::messages::{MsgId, text};
use freya::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ComparisonRow {
    pub caption: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

#[derive(Clone, PartialEq)]
pub(crate) struct ComparisonView {
    pub code: bool,
    pub before: String,
    pub after: String,
    pub rows: Vec<ComparisonRow>,
}
impl Component for ComparisonView {
    fn render(&self) -> impl IntoElement {
        let mut unified = use_state(|| false);
        let mut context = use_state(|| false);
        let mut focused = use_state(|| None::<usize>);
        let changes: Vec<_> = self
            .rows
            .iter()
            .enumerate()
            .filter_map(|(i, row)| (row.before != row.after).then_some(i))
            .collect();
        let selected = (*focused.read()).map(|i| i.min(changes.len().saturating_sub(1)));
        let count = changes.len();
        let colors = t::colors();
        let mut narrow = use_state(|| false);
        let stacked = *unified.read() || *narrow.read();
        let mut body = rect()
            .width(Size::fill())
            .on_sized(move |event: Event<SizedEventData>| {
                narrow.set_if_modified(event.area.width() < 640.);
            })
            .spacing(t::SPACE_LG)
            .child(
                Button::new()
                    .flat()
                    .selected(stacked)
                    .enabled(!*narrow.read())
                    .on_press(move |_| {
                        let next = !*unified.peek();
                        unified.set(next);
                    })
                    .child(text(if stacked {
                        MsgId::WriterSideBySide
                    } else {
                        MsgId::WriterUnified
                    })),
            );
        let mut navigation = rect()
            .horizontal()
            .spacing(t::SPACE_SM)
            .cross_align(Alignment::Center);
        if count > 1 {
            navigation = navigation
                .child(
                    Button::new()
                        .flat()
                        .enabled(selected.is_none_or(|i| i > 0))
                        .on_press(move |_| {
                            focused.set(Some(selected.map_or(count - 1, |i| i.saturating_sub(1))))
                        })
                        .child(text(MsgId::WriterPrevious)),
                )
                .child(label().text(selected.map_or_else(
                    || format!("{}: {count}", text(MsgId::WriterChanges)),
                    |i| format!("{} / {count}", i + 1),
                )))
                .child(
                    Button::new()
                        .flat()
                        .enabled(selected.is_none_or(|i| i + 1 < count))
                        .on_press(move |_| {
                            focused.set(Some(selected.map_or(0, |i| (i + 1).min(count - 1))))
                        })
                        .child(text(MsgId::WriterNext)),
                )
                .maybe_child(selected.map(|_| {
                    Button::new()
                        .flat()
                        .on_press(move |_| focused.set(None))
                        .child(text(MsgId::WriterAllChanges))
                }));
        }
        if changes.len() != self.rows.len() {
            navigation = navigation.child(
                Button::new()
                    .flat()
                    .selected(*context.read())
                    .on_press(move |_| {
                        let next = !*context.peek();
                        context.set(next);
                    })
                    .child(text(if *context.read() {
                        MsgId::WriterHideContext
                    } else {
                        MsgId::WriterShowContext
                    })),
            );
        }
        body = body.child(navigation);
        if count == 0 {
            body = body.child(label().text(text(MsgId::WriterNoChanges)));
        }
        for (index, row) in self.rows.iter().enumerate() {
            if let Some(selected) = selected {
                let Some(&changed) = changes.get(selected) else {
                    continue;
                };
                let nearby_context =
                    *context.read() && row.before == row.after && index.abs_diff(changed) <= 1;
                if index != changed && !nearby_context {
                    continue;
                }
            } else if !*context.read() && row.before == row.after {
                continue;
            }
            let mut pair = rect()
                .width(Size::fill())
                .content(Content::Flex)
                .spacing(t::SPACE_MD);
            if !stacked {
                pair = pair.horizontal();
            }
            for (caption, value, other) in [
                (&self.before, &row.before, &row.after),
                (&self.after, &row.after, &row.before),
            ] {
                let cell = rect()
                    .width(if stacked {
                        Size::fill()
                    } else {
                        Size::flex(1.)
                    })
                    .padding(t::SPACE_LG)
                    .corner_radius(t::RADIUS)
                    .background(colors.surface)
                    .spacing(t::SPACE_SM)
                    .child(label().text(caption.clone()).font_size(t::small()))
                    .child(
                        if let Some(value) = value.as_ref().filter(|v| !v.is_empty()) {
                            diff::render(value, other.as_deref().unwrap_or_default(), self.code)
                        } else {
                            label()
                                .text(text(MsgId::WriterMissingVersion))
                                .color(colors.muted)
                                .into_element()
                        },
                    );
                pair = pair.child(cell);
            }
            body = body.child(
                rect()
                    .width(Size::fill())
                    .spacing(t::SPACE_SM)
                    .child(label().text(row.caption.clone()).font_size(t::small()))
                    .child(pair),
            );
        }
        body
    }
}
