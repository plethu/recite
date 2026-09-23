//! Search text and result navigation share one keyboard contract.
use super::{Button, keyboard, tokens as t};
use freya::prelude::*;

#[derive(Clone, PartialEq)]
pub(crate) struct SearchField {
    pub query: State<String>,
    pub insert_request: Option<State<bool>>,
    pub id: AccessibilityId,
    pub placeholder: String,
    pub active: State<Option<usize>>,
    pub count: usize,
    pub vim: bool,
    pub activate: EventHandler<usize>,
    pub changed: EventHandler<()>,
}
impl Component for SearchField {
    fn render(&self) -> impl IntoElement {
        let mut query = self.query;
        let mut active = self.active;
        let mut normal = use_state(|| false);
        let keys = super::list_keys::ListKeys::new();
        let mut refocus = use_state(|| false);
        let Self {
            id,
            count,
            vim,
            activate,
            changed,
            ..
        } = self.clone();
        let insert_request = self.insert_request;
        use_after_side_effect(move || {
            if let Some(mut requested) = insert_request
                && *requested.read()
            {
                normal.set(false);
                id.request_focus();
                requested.set(false);
            }
        });
        // Input's outside-click handler can clear focus after the clear button runs.
        // Retain this request until the next pointer/key interaction, without a timer.
        use_after_side_effect(move || {
            if *refocus.read() && *Platform::get().focused_accessibility_id.read() != id {
                id.request_focus();
            }
        });
        let validated = changed.clone();
        let cleared = changed.clone();
        use_after_side_effect(move || {
            let _ = query.read();
            active.set_if_modified(None);
        });
        let colors = t::colors();
        let field = rect()
            .background(colors.inset)
            .shadow(super::material::inset(1.))
            .corner_radius(t::RADIUS)
            .border(
                Border::new()
                    .width(if id.is_focused() { t::FOCUS_WIDTH } else { 1. })
                    .fill(if id.is_focused() {
                        colors.accent
                    } else {
                        colors.boundary
                    }),
            )
            .cross_align(Alignment::Center)
            .padding(t::SPACE_XS)
            .on_global_pointer_down(move |_| {
                refocus.set_if_modified(false);
                keys.cancel();
            })
            .horizontal()
            .content(Content::Flex)
            .width(Size::fill())
            .child(
                rect()
                    .padding((0., 2.))
                    .child(crate::controls::Icon::Search.colored(colors.muted)),
            )
            .child(
                Input::new(query)
                    .a11y_id(id)
                    .height(Size::px(t::control_height() - 2. * t::SPACE_XS))
                    .theme_layout(InputLayoutThemePartial {
                        padding: Some(Preference::Specific(Gaps::new(2., 2., 2., 2.))),
                        ..Default::default()
                    })
                    .theme_colors(InputColorsThemePartial {
                        placeholder_color: Some(Preference::Specific(colors.placeholder)),
                        background: Some(Preference::Specific(Color::TRANSPARENT)),
                        focus_background: Some(Preference::Specific(Color::TRANSPARENT)),
                        border_fill: Some(Preference::Specific(Color::TRANSPARENT)),
                        focus_border_fill: Some(Preference::Specific(Color::TRANSPARENT)),
                        ..Default::default()
                    })
                    .width(Size::flex(1.))
                    .placeholder(self.placeholder.clone())
                    .on_validate(move |value: InputValidator| {
                        if *value.text() != *query.peek() {
                            validated.call(());
                        }
                    })
                    .on_pre_key_down(move |event: Event<KeyboardEventData>| {
                        refocus.set_if_modified(false);
                        let current = *active.peek();
                        if let Some(next) =
                            keys.navigate(&event, vim && *normal.peek(), current, count)
                        {
                            if let Some(next) = next {
                                active.set(Some(next));
                            }
                            event.stop_propagation();
                            event.prevent_default();
                            return false;
                        }
                        if event.modifiers.is_empty() {
                            match &event.key {
                                Key::Named(NamedKey::Enter) => {
                                    if count > 0 {
                                        activate.call(active.peek().unwrap_or(0).min(count - 1));
                                    }
                                    event.stop_propagation();
                                    return false;
                                }
                                Key::Named(NamedKey::Escape) => {
                                    if vim && !*normal.peek() {
                                        normal.set(true);
                                    } else {
                                        query.set(String::new());
                                        active.set(None);
                                        changed.call(());
                                    }
                                    event.stop_propagation();
                                    return false;
                                }
                                Key::Character(key)
                                    if vim
                                        && *normal.peek()
                                        && (key == "i" || key == "a" || key == "/") =>
                                {
                                    normal.set(false);
                                    event.stop_propagation();
                                    return false;
                                }
                                _ => {}
                            }
                        }
                        if vim && *normal.peek() {
                            if keyboard::vim_workspace_key(&event) {
                                return false;
                            }
                            event.stop_propagation();
                            false
                        } else {
                            crate::closing::text_input_key(event)
                        }
                    }),
            )
            .maybe_child((!query.read().is_empty()).then(|| {
                Button::new()
                    .flat()
                    .compact()
                    .width(Size::px(t::control_height() - 2. * t::SPACE_XS))
                    .named(format!("Clear {}", self.placeholder))
                    .on_press(move |_| {
                        query.set(String::new());
                        active.set(None);
                        cleared.call(());
                        normal.set(false);
                        id.request_focus();
                        refocus.set(true);
                    })
                    .child(crate::controls::Icon::Close.colored(colors.muted))
            }));
        rect()
            .width(Size::fill())
            .spacing(t::SPACE_XS)
            .child(field)
            .maybe_child(vim.then(|| keys.mode(*normal.read())))
    }
}

#[cfg(test)]
mod tests;
