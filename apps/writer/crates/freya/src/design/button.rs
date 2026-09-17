//! All button variants share focus, pointer, keyboard and disabled behaviour.
use super::tokens as t;
use freya::prelude::*;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Secondary,
    Quiet,
    Primary,
}

#[derive(Clone, Copy, PartialEq)]
enum Semantics {
    Button,
    Radio,
    Option,
    MenuItem,
}

#[derive(Clone, PartialEq)]
pub(crate) struct Button {
    children: Vec<Element>,
    action: Option<EventHandler<Event<PressEventData>>>,
    hover_changed: Option<EventHandler<bool>>,
    id: Option<AccessibilityId>,
    name: Option<String>,
    enabled: bool,
    selected: Option<bool>,
    checked: Option<bool>,
    semantics: Semantics,
    expanded: Option<bool>,
    kind: Kind,
    width: Size,
    key: DiffKey,
}

impl Button {
    pub fn new() -> Self {
        Self {
            children: vec![],
            action: None,
            hover_changed: None,
            id: None,
            name: None,
            enabled: true,
            selected: None,
            checked: None,
            semantics: Semantics::Button,
            expanded: None,
            kind: Kind::Secondary,
            width: Size::auto(),
            key: DiffKey::None,
        }
    }
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = Some(expanded);
        self
    }
    pub fn menu_item(mut self) -> Self {
        self.semantics = Semantics::MenuItem;
        self.kind = Kind::Quiet;
        self
    }
    pub fn option(mut self, selected: bool) -> Self {
        self.semantics = Semantics::Option;
        self.selected = Some(selected);
        self
    }
    pub fn radio(mut self, selected: bool) -> Self {
        self.semantics = Semantics::Radio;
        self.selected = Some(selected);
        self
    }
    pub fn checkable(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }
    pub fn on_hover_changed(mut self, action: impl Into<EventHandler<bool>>) -> Self {
        self.hover_changed = Some(action.into());
        self
    }
    pub fn flat(mut self) -> Self {
        self.kind = Kind::Quiet;
        self
    }
    pub fn filled(mut self) -> Self {
        self.kind = Kind::Primary;
        self
    }
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = Some(selected);
        self
    }
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    pub fn width(mut self, width: Size) -> Self {
        self.width = width;
        self
    }
    pub fn a11y_id(mut self, id: AccessibilityId) -> Self {
        self.id = Some(id);
        self
    }
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
    pub fn on_press(mut self, action: impl Into<EventHandler<Event<PressEventData>>>) -> Self {
        self.action = Some(action.into());
        self
    }
}
impl ChildrenExt for Button {
    fn get_children(&mut self) -> &mut Vec<Element> {
        &mut self.children
    }
}
impl KeyExt for Button {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}
impl Component for Button {
    fn render(&self) -> impl IntoElement {
        let colors = t::colors();
        let fallback = use_a11y();
        let id = self.id.unwrap_or(fallback);
        let mut hovered = use_state(|| false);
        let mut pressed = use_state(|| false);
        let action = self.action.clone();
        let enter = self.hover_changed.clone();
        let leave = self.hover_changed.clone();
        let focused = id.is_focused();
        let emphasized = self.selected == Some(true) || (self.enabled && *hovered.read());
        let background = if !self.enabled {
            colors.surface
        } else if *pressed.read() {
            colors.pressed
        } else if self.semantics == Semantics::Radio && self.selected == Some(true) {
            colors.accent
        } else if emphasized {
            colors.hover
        } else {
            match self.kind {
                Kind::Quiet => Color::TRANSPARENT,
                Kind::Secondary => colors.surface,
                Kind::Primary => colors.accent,
            }
        };
        let color = if (self.kind == Kind::Primary && !emphasized)
            || (self.semantics == Semantics::Radio && self.selected == Some(true))
        {
            colors.on_accent
        } else {
            colors.ink
        };
        let role = if self.semantics == Semantics::MenuItem {
            AccessibilityRole::MenuItem
        } else if self.semantics == Semantics::Option {
            AccessibilityRole::ListBoxOption
        } else if self.semantics == Semantics::Radio {
            AccessibilityRole::RadioButton
        } else if self.checked.is_some() {
            AccessibilityRole::CheckBox
        } else {
            AccessibilityRole::Button
        };
        let border_color = if focused {
            colors.accent
        } else if self.kind == Kind::Secondary {
            colors.rule
        } else {
            Color::TRANSPARENT
        };
        let mut control = rect()
            .a11y_id(id)
            .a11y_focusable(self.enabled)
            .a11y_role(role)
            .width(self.width.clone())
            .min_height(Size::px(t::CONTROL_HEIGHT - 2. * t::SPACE_SM))
            .padding(Gaps::new(
                t::SPACE_XS,
                t::SPACE_SM,
                t::SPACE_XS,
                t::SPACE_SM,
            ))
            .corner_radius(t::RADIUS)
            .main_align(Alignment::Center)
            .cross_align(Alignment::Center)
            .background(background)
            .color(if self.enabled { color } else { colors.muted })
            .border(
                Border::new()
                    .width(if focused { t::FOCUS_WIDTH } else { 1. })
                    .alignment(BorderAlignment::Inner)
                    .fill(border_color),
            )
            .cursor(if self.enabled {
                CursorIcon::Pointer
            } else {
                CursorIcon::Default
            });
        if let Some(name) = &self.name {
            control = control.a11y_alt(name.clone());
        }
        if self.semantics == Semantics::Option {
            let selected = self.selected.unwrap_or(false);
            control = control.a11y_builder(move |node| node.set_selected(selected));
        } else if let Some(checked) = self.checked.or(self.selected) {
            control = control.a11y_builder(move |node| {
                node.set_toggled(if checked {
                    accesskit::Toggled::True
                } else {
                    accesskit::Toggled::False
                })
            });
        }
        if let Some(expanded) = self.expanded {
            control = control.a11y_builder(move |node| node.set_expanded(expanded));
        }
        if self.enabled {
            control = control
                .on_pointer_enter(move |_| { hovered.set(true); if let Some(enter) = &enter { enter.call(true); } })
                .on_pointer_leave(move |_| { hovered.set(false); if let Some(leave) = &leave { leave.call(false); } })
                .on_pointer_down(move |event: Event<PointerEventData>| {
                    if event.is_primary() { pressed.set(true); event.stop_propagation(); }
                })
                .on_global_pointer_press(move |_| { pressed.set_if_modified(false); })
                .on_all_press(move |event: Event<PressEventData>| {
                    if matches!(event.data(), PressEventData::Mouse(data) if data.button != Some(MouseButton::Left)) { return; }
                    event.stop_propagation(); id.request_focus();
                    if let Some(action) = &action { action.call(event); }
                });
        }
        control.children(self.children.clone())
    }
    fn render_key(&self) -> DiffKey {
        self.key.clone()
    }
}
