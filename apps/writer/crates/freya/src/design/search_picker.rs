//! A single-field picker with an anchored, virtualised result list.
use super::{Button, keyboard, tokens as t};
use freya::prelude::*;
use std::sync::Arc;

#[derive(Clone, PartialEq)]
pub(crate) struct PickerOption {
    pub value: String,
    pub title: String,
    pub detail: String,
    /// Human-facing tag or context, independent of the opaque selection value.
    pub annotation: String,
}

#[derive(Clone, PartialEq)]
pub(crate) struct SearchPicker {
    pub id: AccessibilityId,
    pub input_id: AccessibilityId,
    pub name: String,
    pub placeholder: String,
    pub empty_hint: String,
    pub no_matches: String,
    pub selected: String,
    pub query: State<String>,
    pub open: State<bool>,
    pub options: Arc<Vec<PickerOption>>,
    pub choose: EventHandler<PickerOption>,
    pub vim: bool,
    pub enabled: bool,
}

impl Component for SearchPicker {
    fn render(&self) -> impl IntoElement {
        let mut open = self.open;
        let mut query = self.query;
        let trigger = self.id;
        let id = self.input_id;
        let list_id = use_a11y();
        let active_id = use_a11y();
        let mut normal = use_state(|| false);
        let keys = super::list_keys::ListKeys::new();
        let mut active = use_state(|| 0usize);
        let mut anchor = use_state(|| None::<Area>);
        let mut popup = use_state(|| None::<Area>);
        let mut scroll = use_scroll_controller(ScrollConfig::default);
        let options = self.options.clone();
        let choose = self.choose.clone();
        let vim = self.vim;
        let colors = t::colors();
        let mut previous_query = use_state(|| None::<String>);
        use_after_side_effect(move || {
            let current = query.read().clone();
            if previous_query.peek().as_ref() == Some(&current) {
                return;
            }
            previous_query.set(Some(current));
            active.set_if_modified(0);
            scroll.scroll_to(ScrollPosition::Start, Direction::Vertical);
        });
        use_after_side_effect(move || {
            if *open.read() {
                normal.set(false);
                id.request_focus();
            }
        });
        use_after_side_effect(move || {
            if *open.read() && Platform::get().focused_accessibility_id.read().0 == 0 {
                id.request_focus();
            }
        });
        let field = if *open.read() {
            Input::new(query)
                .auto_focus(true)
                .a11y_id(id)
                .width(Size::fill())
                .placeholder(self.placeholder.clone())
                .on_pre_key_down(move |event: Event<KeyboardEventData>| {
                    if keyboard::submit_key(&event) {
                        event.stop_propagation();
                        event.prevent_default();
                        return false;
                    }
                    if event.key == Key::Named(NamedKey::Escape) {
                        keys.cancel();
                        event.stop_propagation();
                        event.prevent_default();
                        if vim && !*normal.peek() {
                            normal.set(true);
                        } else {
                            open.set(false);
                            trigger.request_focus();
                        }
                        return false;
                    }
                    if event.key == Key::Named(NamedKey::Tab) {
                        keys.cancel();
                        open.set(false);
                        return false;
                    }
                    let current = *active.peek();
                    if let Some(next) =
                        keys.navigate(&event, vim && *normal.peek(), Some(current), options.len())
                    {
                        if let Some(next) = next {
                            active.set(next);
                            scroll.scroll_to_y(
                                -((next.saturating_sub(2) * t::picker_row_height() as usize)
                                    as i32),
                            );
                        }
                        event.stop_propagation();
                        event.prevent_default();
                        return false;
                    }
                    if event.key == Key::Named(NamedKey::Enter) && event.modifiers.is_empty() {
                        if let Some(option) = options.get(*active.peek()) {
                            choose.call(option.clone());
                            open.set(false);
                            trigger.request_focus();
                        }
                        event.stop_propagation();
                        event.prevent_default();
                        return false;
                    }
                    if vim && *normal.peek() {
                        if matches!(&event.key, Key::Character(key) if key == "i" || key == "/") {
                            normal.set(false);
                        }
                        event.stop_propagation();
                        event.prevent_default();
                        return false;
                    }
                    crate::closing::text_input_key(event)
                })
                .into_element()
        } else {
            Button::new()
                .a11y_id(trigger)
                .named(self.name.clone())
                .enabled(self.enabled)
                .expanded(false)
                .width(Size::fill())
                .on_press(move |_| {
                    query.set(String::new());
                    open.set(true);
                })
                .child(
                    rect()
                        .horizontal()
                        .content(Content::Flex)
                        .width(Size::fill())
                        .child(
                            label()
                                .width(Size::flex(1.))
                                .text(if self.selected.is_empty() {
                                    self.name.clone()
                                } else {
                                    self.selected.clone()
                                }),
                        )
                        .child(crate::controls::Icon::ChevronDown.colored(colors.muted)),
                )
                .into_element()
        };
        let expanded = *open.read();
        let has_options = !self.options.is_empty();
        let mut root = rect()
            .on_global_pointer_down(move |_| keys.cancel())
            .a11y_role(AccessibilityRole::ComboBox)
            .a11y_alt(self.name.clone())
            .a11y_builder(move |node| {
                node.set_expanded(expanded);
                if expanded {
                    node.set_controls(vec![list_id]);
                    if has_options {
                        node.set_active_descendant(active_id);
                    }
                }
            })
            .width(Size::fill())
            .height(Size::px(t::control_height()))
            .on_sized(move |event: Event<SizedEventData>| anchor.set_if_modified(Some(event.area)))
            .child(field);
        if *self.open.read()
            && let Some(area) = *anchor.read()
        {
            let options = self.options.clone();
            let choose = self.choose.clone();
            let count = options.len();
            let height = count.min(5) as f32 * t::picker_row_height();
            let hint = if count == 0 {
                Some(if self.query.read().trim().is_empty() {
                    self.empty_hint.clone()
                } else {
                    self.no_matches.clone()
                })
            } else if vim {
                Some(
                    if keys.pending() {
                        "g → g first · Esc cancel"
                    } else if *normal.read() {
                        "NORMAL · gg G j k / ↑ ↓ · Enter"
                    } else {
                        "INSERT · ↑ ↓ · Enter · Esc → NORMAL"
                    }
                    .into(),
                )
            } else {
                None
            };
            let popup_height = height
                + 2. * t::SPACE_XS
                + if hint.is_some() {
                    t::control_height()
                } else {
                    0.
                };
            let window_size = *Platform::get().root_size.read();
            let window = window_size.height;
            let width = area
                .width()
                .min(t::PICKER_MAX_WIDTH)
                .min((window_size.width - 16.).max(0.));
            let left = area.min_x().min((window_size.width - width - 8.).max(8.));
            let top = if area.max_y() + popup_height + 6. > window {
                (area.min_y() - popup_height - 6.).max(8.)
            } else {
                area.max_y() + 6.
            };
            let selected = *active.read();
            let list = VirtualScrollView::new_with_data(
                (options, selected),
                move |item, (options, selected)| {
                    let option = options[item.index].clone();
                    let choose = choose.clone();
                    let mut button = Button::new();
                    if item.index == *selected {
                        button = button.a11y_id(active_id);
                    }
                    rect()
                        .key(item.index)
                        .height(Size::px(t::picker_row_height()))
                        .width(Size::fill())
                        .child(
                            button
                                .flat()
                                .option(item.index == *selected)
                                .width(Size::fill())
                                .named(format!("{} · {}", option.title, option.value))
                                .on_press(move |_| {
                                    choose.call(option.clone());
                                    open.set(false);
                                    trigger.request_focus();
                                })
                                .child(option_row(&options[item.index], colors.muted)),
                        )
                        .into_element()
                },
            )
            .length(count)
            .item_size(t::picker_row_height())
            .scroll_controller(scroll)
            .scroll_with_arrows(false)
            .height(Size::px(height))
            .width(Size::fill());
            root = root.child(super::motion::Appear {
                area: Area::new((left, top).into(), Size2D::new(width, popup_height)),
                content: rect()
                    .width(Size::fill())
                    .height(Size::fill())
                    .background(colors.floating)
                    .padding(t::SPACE_XS)
                    .shadow((0., 6., 20., 0., colors.shadow))
                    .border(Border::new().width(1.).fill(colors.rule))
                    .corner_radius(t::DIALOG_RADIUS)
                    .on_sized(move |event: Event<SizedEventData>| {
                        popup.set_if_modified(Some(event.area))
                    })
                    .on_global_pointer_press(move |event: Event<PointerEventData>| {
                        let point = event.global_location().to_f32();
                        if !anchor.peek().is_some_and(|a| a.contains(point))
                            && !popup.peek().is_some_and(|a| a.contains(point))
                        {
                            open.set_if_modified(false);
                        }
                    })
                    .child(
                        rect()
                            .width(Size::fill())
                            .height(Size::px(height))
                            .a11y_id(list_id)
                            .a11y_role(AccessibilityRole::ListBox)
                            .a11y_alt(self.name.clone())
                            .child(list),
                    )
                    .maybe_child(hint.map(|hint| {
                        label()
                            .text(hint)
                            .font_size(t::small())
                            .color(colors.muted)
                            .padding(t::SPACE_SM)
                    }))
                    .into_element(),
            });
        }
        root
    }
}

fn option_row(option: &PickerOption, muted: Color) -> Rect {
    let mut name = rect()
        .width(Size::flex(1.))
        .child(label().text(option.title.clone()));
    if !option.detail.is_empty() && option.detail != option.title {
        name = name.child(
            label()
                .text(option.detail.clone())
                .font_size(t::small())
                .color(muted),
        );
    }
    rect()
        .horizontal()
        .content(Content::Flex)
        .width(Size::fill())
        .cross_align(Alignment::Center)
        .spacing(t::SPACE_SM)
        .child(name)
        .maybe_child((!option.annotation.is_empty()).then(|| {
            label()
                .text(option.annotation.clone())
                .font_size(t::small())
                .color(muted)
        }))
}
