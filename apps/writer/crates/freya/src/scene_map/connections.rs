//! Connections retain direction, reply and endpoints beside explicit map navigation.
use crate::design::tokens as t;
use crate::{editing::Writer, palette};
use freya::prelude::*;
use recite_writer_model::SceneLink;

pub(super) fn panel(
    writer: Writer,
    selected: &str,
    links: &[SceneLink],
    mut active: State<Option<SceneLink>>,
    mut open: State<bool>,
) -> Element {
    let mut groups = rect()
        .width(Size::fill())
        .horizontal()
        .content(Content::Flex)
        .spacing(t::SPACE_MD);
    for (outgoing, heading) in [(true, "Outgoing"), (false, "Incoming")] {
        let items: Vec<_> = links
            .iter()
            .filter(|link| {
                if outgoing {
                    link.origin == selected
                } else {
                    link.destination == selected && link.origin != selected
                }
            })
            .cloned()
            .collect();
        let count = items.len();
        let group = VirtualScrollView::new_with_data(items, move |item, links| {
            rect()
                .key(item.index)
                .width(Size::fill())
                .height(Size::px(64.))
                .overflow(Overflow::Clip)
                .child(Connection {
                    writer,
                    link: links[item.index].clone(),
                    outgoing,
                    active,
                })
                .into_element()
        })
        .key(format!("{selected}:{heading}"))
        .length(count)
        .item_size(64.)
        .width(Size::fill())
        .height(Size::px(if count == 0 { 0. } else { 136. }));
        groups = groups.child(
            rect()
                .width(Size::flex(1.))
                .a11y_role(AccessibilityRole::List)
                .a11y_alt(format!("{heading} connections"))
                .spacing(t::SPACE_SM)
                .child(label().text(heading).font_size(t::small()))
                .child(
                    rect()
                        .width(Size::fill())
                        .child(group)
                        .maybe_child((count == 0).then(|| {
                            label()
                                .text(crate::messages::text(crate::messages::MsgId::WriterGuiNone))
                                .font_size(t::small())
                        })),
                ),
        );
    }
    rect()
        .width(Size::fill())
        .spacing(t::SPACE_SM)
        .child(
            crate::design::Button::new()
                .flat()
                .named(crate::messages::text(
                    crate::messages::MsgId::WriterGuiConnections,
                ))
                .expanded(*open.read())
                .on_press(move |_| {
                    let next = !*open.peek();
                    open.set(next);
                    active.set(None);
                })
                .child(format!(
                    "{} Connections · {}",
                    if *open.read() { "▾" } else { "▸" },
                    palette::display_name(selected)
                )),
        )
        .maybe_child((*open.read()).then(|| {
            label()
                .text(crate::messages::text(
                    crate::messages::MsgId::WriterGuiConnectionHelp,
                ))
                .font_size(t::small())
                .color(palette::muted(writer.dark))
        }))
        .maybe_child((*open.read()).then_some(groups))
        .into_element()
}

#[derive(Clone)]
struct Connection {
    writer: Writer,
    link: SceneLink,
    outgoing: bool,
    active: State<Option<SceneLink>>,
}

impl PartialEq for Connection {
    fn eq(&self, other: &Self) -> bool {
        self.link == other.link
            && self.outgoing == other.outgoing
            && self.writer.dark == other.writer.dark
    }
}

impl Component for Connection {
    fn render(&self) -> impl IntoElement {
        let colors = t::colors();
        let focus = use_a11y();
        let mut active = self.active;
        let mut hovered = use_state(|| false);
        let link = self.link.clone();
        use_side_effect(move || {
            if focus.is_focused() || *hovered.read() {
                active.set_if_modified(Some(link.clone()));
            } else if active.peek().as_ref() == Some(&link) {
                active.set(None);
            }
        });
        let mut writer = self.writer;
        let target = if self.outgoing {
            &self.link.destination
        } else {
            &self.link.origin
        };
        let ending = target == "END";
        let target = target.clone();
        let destination = if self.link.destination == "END" {
            "End of conversation".into()
        } else {
            palette::display_name(&self.link.destination)
        };
        let endpoints = format!(
            "{} → {destination}",
            palette::display_name(&self.link.origin)
        );
        let mut detail = self.link.label.clone();
        if self.link.returning {
            detail.push_str(" · return");
        }
        if self.link.conditional {
            detail.push_str(" · conditional");
        }
        let action = format!("Show {} on map", palette::display_name(&target));
        let content = rect()
            .width(Size::fill())
            .spacing(t::SPACE_XS)
            .child(
                rect()
                    .horizontal()
                    .width(Size::fill())
                    .content(Content::Flex)
                    .child(
                        label()
                            .text(endpoints.clone())
                            .max_lines(1)
                            .text_overflow(TextOverflow::Ellipsis)
                            .width(Size::flex(1.))
                            .font_size(t::body()),
                    )
                    .maybe_child((!ending).then(|| label().text("›").font_size(t::body()))),
            )
            .child(
                label()
                    .text(if detail.chars().count() > 160 {
                        format!("{}…", detail.chars().take(160).collect::<String>())
                    } else {
                        detail.clone()
                    })
                    .font_size(t::small())
                    .max_lines(2)
                    .text_overflow(TextOverflow::Ellipsis)
                    .color(colors.muted),
            );
        if ending {
            rect()
                .width(Size::fill())
                .padding(t::SPACE_SM)
                .a11y_role(AccessibilityRole::ListItem)
                .a11y_alt(format!("{endpoints}. {detail}"))
                .child(content)
                .into_element()
        } else {
            crate::design::Button::new()
                .flat()
                .a11y_id(focus)
                .width(Size::fill())
                .named(format!("{action}. {endpoints}. {detail}"))
                .on_hover_changed(move |value| hovered.set(value))
                .on_press(move |_| {
                    active.set(None);
                    writer.selection.set(Some(target.clone()));
                    writer.map_focus.request_focus();
                })
                .child(content)
                .into_element()
        }
    }
}
