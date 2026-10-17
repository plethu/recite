//! All button variants share focus, pointer, keyboard and disabled behaviour.
mod semantics;
use super::tokens as t;
use freya::prelude::*;
use semantics::Semantics;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// Quiet at rest; a stamp in a `StampScope` or with `stamp()`; filled ink when switched on.
    Secondary,
    /// List rows and toolbar text actions; selection is the second-ink plate.
    Quiet,
    Primary,
    /// Opens a picker or acts as a select, so it looks like an input.
    Field,
}

#[cfg(test)]
mod tests;

#[derive(Clone, PartialEq)]
pub(crate) struct Button {
    children: Vec<Element>,
    action: Option<EventHandler<Event<PressEventData>>>,
    hover_changed: Option<EventHandler<bool>>,
    id: Option<AccessibilityId>,
    name: Option<String>,
    shortcut: Option<String>,
    enabled: bool,
    highlighted: bool,
    semantics: Semantics,
    expanded: Option<bool>,
    kind: Kind,
    segment: bool,
    stamp: bool,
    destination: bool,
    plate_bleed: f32,
    row_gutter: f32,
    width: Size,
    compact: bool,
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
            shortcut: None,
            enabled: true,
            highlighted: false,
            semantics: Semantics::Button,
            expanded: None,
            kind: Kind::Secondary,
            segment: false,
            stamp: false,
            destination: false,
            plate_bleed: 0.,
            row_gutter: 0.,
            width: Size::auto(),
            compact: false,
            key: DiffKey::None,
        }
    }
    pub fn shortcut(mut self, shortcut: String) -> Self {
        self.shortcut = (!shortcut.is_empty()).then_some(shortcut);
        self
    }
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = Some(expanded);
        self
    }
    pub fn tab(mut self, selected: bool) -> Self {
        self.semantics = Semantics::Tab { selected };
        self.highlighted = selected;
        self.kind = Kind::Quiet;
        self
    }
    pub fn menu_item(mut self) -> Self {
        self.semantics = Semantics::MenuItem;
        self.kind = Kind::Quiet;
        self
    }
    pub fn option(mut self, selected: bool) -> Self {
        self.semantics = Semantics::Option { selected };
        self.highlighted = selected;
        self
    }
    pub fn radio(mut self, selected: bool) -> Self {
        self.semantics = Semantics::Radio { selected };
        self.highlighted = selected;
        self
    }
    /// A radio drawn inside `Segments`, whose shared indicator supplies the selected fill.
    pub fn segment(mut self, selected: bool) -> Self {
        self = self.flat().radio(selected);
        self.segment = true;
        self
    }
    pub fn checkable(mut self, checked: bool) -> Self {
        self.semantics = Semantics::CheckBox { checked };
        self
    }
    pub fn toggle(mut self, pressed: bool) -> Self {
        self.semantics = Semantics::Toggle { pressed };
        self.highlighted = pressed;
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
    /// A standalone secondary action outside a dialog action row, such as an empty state.
    pub fn stamp(mut self) -> Self {
        self.stamp = true;
        self
    }
    /// How far a selected row's plate extends left of the row to reach its container's edge.
    pub fn plate_bleed(mut self, bleed: f32) -> Self {
        self.plate_bleed = bleed;
        self
    }
    /// Reserve a left gutter in a list row, always, so the selection marker never moves its text.
    pub fn row_gutter(mut self) -> Self {
        self.row_gutter = t::ROW_GUTTER;
        self
    }
    /// A route to another beat: text-safe teal, ink while pressed.
    pub fn destination(mut self) -> Self {
        self.kind = Kind::Quiet;
        self.destination = true;
        self
    }
    /// A select or picker trigger: inset fill, boundary stroke; callers supply the chevron.
    pub fn field(mut self) -> Self {
        self.kind = Kind::Field;
        self
    }
    /// Emphasize a surface without claiming an accessible selection or toggle.
    pub fn highlighted(mut self, highlighted: bool) -> Self {
        self.highlighted = highlighted;
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
/// Which filled ink a control shows: plum for the one primary action, ink for switched-on state.
#[derive(Clone, Copy, PartialEq)]
enum Fill {
    None,
    Accent,
    Ink,
}

/// Dialog action rows and empty states draw standalone secondary actions as stamps.
#[derive(Clone, Copy)]
pub(crate) struct StampScope;

impl Button {
    fn fill(&self) -> Fill {
        match self.kind {
            Kind::Primary => Fill::Accent,
            Kind::Secondary if self.highlighted => Fill::Ink,
            _ if self.segment && self.highlighted => Fill::Ink,
            _ => Fill::None,
        }
    }

    /// Quiet list rows show selection as the second-ink plate with a bar, not a grey fill.
    fn plated(&self) -> bool {
        self.kind == Kind::Quiet
            && self.highlighted
            && !self.segment
            && !self.semantics.is_tab()
            && self.enabled
    }

    /// The active document tab is marked by a second-ink rule along its lower edge.
    fn underlined(&self) -> bool {
        self.semantics.is_tab() && self.highlighted && self.enabled
    }

    fn background(&self, colors: &super::palette::Palette, hovered: bool, pressed: bool) -> Color {
        let fill = self.fill();
        if !self.enabled {
            match self.kind {
                Kind::Primary | Kind::Field => colors.inset,
                _ => Color::TRANSPARENT,
            }
        } else if self.segment {
            // The group's travelling indicator is the selected segment's fill.
            if self.highlighted {
                Color::TRANSPARENT
            } else if pressed {
                colors.pressed
            } else if hovered {
                colors.hover
            } else {
                Color::TRANSPARENT
            }
        } else {
            let lift = if pressed {
                t::STAMP_PRESSED
            } else if hovered {
                t::STAMP_HOVER
            } else {
                0.
            };
            match fill {
                Fill::Accent => Color::lerp(colors.accent, colors.on_accent, lift * 0.7),
                Fill::Ink => colors.stamp_state(lift),
                Fill::None if pressed => colors.pressed,
                // The thumb-index plate is drawn as its own layer so it can bleed past the row.
                Fill::None if self.plated() => Color::TRANSPARENT,
                Fill::None if hovered => colors.hover,
                Fill::None if self.kind == Kind::Field => colors.inset,
                Fill::None => Color::TRANSPARENT,
            }
        }
    }

    fn border_color(
        &self,
        colors: &super::palette::Palette,
        focused: bool,
        stamped: bool,
    ) -> Color {
        if focused {
            match self.fill() {
                Fill::Accent => colors.on_accent,
                Fill::Ink => colors.on_stamp,
                Fill::None => colors.accent,
            }
        } else if self.kind == Kind::Field {
            colors.boundary
        } else if stamped && self.kind == Kind::Secondary {
            if self.enabled {
                colors.stamp
            } else {
                colors.rule
            }
        } else {
            Color::TRANSPARENT
        }
    }

    fn border_width(&self, focused: bool, stamped: bool) -> f32 {
        if focused {
            t::FOCUS_WIDTH
        } else if stamped && self.kind == Kind::Secondary {
            t::STROKE_WIDTH
        } else {
            1.
        }
    }

    fn text_color(&self, colors: &super::palette::Palette, pressed: bool) -> Color {
        if !self.enabled {
            return colors.muted;
        }
        match self.fill() {
            Fill::Accent => colors.on_accent,
            Fill::Ink => colors.on_stamp,
            Fill::None if self.destination => colors.destination(pressed),
            Fill::None => colors.ink,
        }
    }

    fn with_accessibility(&self, control: Rect) -> Rect {
        let enabled = self.enabled;
        let shortcut = self.shortcut.clone();
        let name = self.name.clone().or_else(|| content_name(&self.children));
        let expanded = self.expanded;
        self.semantics
            .apply(control)
            .a11y_focusable(enabled)
            .a11y_builder(move |node| {
                if !enabled {
                    node.set_disabled();
                }
                if let Some(shortcut) = shortcut {
                    node.set_keyboard_shortcut(shortcut);
                }
                if let Some(name) = name {
                    node.set_label(name);
                }
                if let Some(expanded) = expanded {
                    node.set_expanded(expanded);
                }
            })
    }
}

impl Component for Button {
    fn render(&self) -> impl IntoElement {
        let colors = t::colors();
        let fallback = use_a11y();
        let id = self.id.unwrap_or(fallback);
        let mut area = use_state(|| None);
        super::focus_scroll::use_reveal(id, area);
        let mut hovered = use_state(|| false);
        let mut pressed = use_state(|| false);
        let action = self.action.clone();
        let enter = self.hover_changed.clone();
        let leave = self.hover_changed.clone();
        let focused = use_focus(id)() == Focus::Keyboard;
        let selected = self.highlighted;
        let filled = self.fill() != Fill::None;
        let segment = self.segment;
        let in_stamp_scope = use_try_consume::<StampScope>().is_some();
        let stamped = self.stamp || in_stamp_scope;
        let background = self.background(&colors, *hovered.read(), *pressed.read());
        let color = self.text_color(&colors, *pressed.read());
        let background = super::motion::surface(background, *pressed.read());
        let down = self.enabled && *pressed.read();
        let depth = super::motion::travel(
            if down { 1. } else { 0. },
            if down { 0 } else { 80 },
            freya::animation::Function::Cubic,
        );
        let border_color = self.border_color(&colors, focused, stamped);
        let border_width = self.border_width(focused, stamped);
        let mut control = rect()
            .horizontal()
            .on_sized(move |event: Event<SizedEventData>| area.set_if_modified(Some(event.area)))
            .a11y_id(id)
            .width(self.width.clone())
            .min_height(Size::px(if self.compact || segment {
                t::control_height() - 2. * t::SPACE_XS
            } else {
                t::control_height()
            }))
            .font_size(t::body())
            .font_weight(if self.plated() {
                FontWeight::SEMI_BOLD
            } else if selected || filled || segment {
                FontWeight::MEDIUM
            } else {
                FontWeight::NORMAL
            })
            .padding((0., t::SPACE_SM, 0., t::SPACE_SM + self.row_gutter))
            .corner_radius(t::RADIUS)
            .main_align(Alignment::Center)
            .cross_align(Alignment::Center)
            .background(background)
            .shadow(super::material::inset(depth))
            .color(color)
            .border(if self.underlined() && !focused {
                Border::new()
                    .width(BorderWidth {
                        bottom: t::FOCUS_WIDTH,
                        ..Default::default()
                    })
                    .alignment(BorderAlignment::Inner)
                    .fill(colors.teal)
            } else {
                Border::new()
                    .width(border_width)
                    .alignment(BorderAlignment::Inner)
                    .fill(border_color)
            })
            .cursor(if self.enabled {
                CursorIcon::Pointer
            } else {
                CursorIcon::Default
            });
        control = self.with_accessibility(control);
        if self.plated() {
            // Thumb index: the plate runs flush to the container's left edge, square there and
            // rounded on the right, with a teal diamond in the gutter. Absolute children are
            // placed from the content box, so offsets start at the left padding.
            let gutter = t::SPACE_SM + self.row_gutter + self.plate_bleed;
            let size = (*area.read()).map(|area: Area| area.size);
            let (width, height) = size.map_or((0., 0.), |size| (size.width, size.height));
            control = control
                .child(
                    rect()
                        .position(Position::new_absolute().left(-gutter).top(0.))
                        .width(Size::px(width + self.plate_bleed))
                        .height(Size::px(height))
                        .corner_radius((0., t::RADIUS, t::RADIUS, 0.))
                        .background(colors.plate),
                )
                .maybe_child((gutter >= t::ROW_GUTTER + t::SPACE_SM).then(|| {
                    rect()
                        .position(
                            Position::new_absolute()
                                .left(-gutter / 2. - t::MARKER_SIZE / 2.)
                                .top((height - t::MARKER_SIZE) / 2.),
                        )
                        .child(super::material::diamond(colors.teal, t::MARKER_SIZE))
                }));
        }
        if self.enabled {
            control = control
                .on_pointer_enter(move |_| { hovered.set(true); if let Some(enter) = &enter { enter.call(true); } })
                .on_pointer_leave(move |_| { hovered.set(false); pressed.set(false); if let Some(leave) = &leave { leave.call(false); } })
                .on_pointer_down(move |event: Event<PointerEventData>| {
                    if event.is_primary() { pressed.set(true); event.stop_propagation(); }
                })
                .on_global_pointer_press(move |_| { pressed.set_if_modified(false); })
                .on_global_key_up(move |event: Event<KeyboardEventData>| {
                    if event.is_press_event() { pressed.set_if_modified(false); }
                })
                .on_all_press(move |event: Event<PressEventData>| {
                    if matches!(event.data(), PressEventData::Mouse(data) if data.button != Some(MouseButton::Left)) { return; }
                    // Form submission must not also activate the focused checkbox or action.
                    if matches!(event.data(), PressEventData::Keyboard(data) if super::keyboard::submit_key(data)) { return; }
                    if matches!(event.data(), PressEventData::Keyboard(_)) { pressed.set_if_modified(true); }
                    event.stop_propagation(); id.request_focus();
                    if let Some(action) = &action { action.call(event); }
                });
        }
        if self.enabled {
            control = control.maybe_child(
                self.shortcut
                    .as_deref()
                    .and_then(|binding| crate::commands::hint(binding, *area.read())),
            );
        }
        control.child(
            rect()
                .horizontal()
                .width(if self.width == Size::auto() {
                    Size::auto()
                } else {
                    Size::fill()
                })
                .cross_align(Alignment::Center)
                .main_align(Alignment::Center)
                .offset_y(depth)
                .children(self.children.clone()),
        )
    }
    fn render_key(&self) -> DiffKey {
        self.key.clone()
    }
}

// A press-depth wrapper must not hide a text-only button's accessible name.
// Custom components still supply an explicit name through `named`.
fn content_name(children: &[Element]) -> Option<String> {
    let words: Vec<_> = children
        .iter()
        .filter_map(|child| {
            let Element::Element {
                element, elements, ..
            } = child
            else {
                return None;
            };
            let mut node = element.accessibility().builder.clone();
            element.finish_accessibility(&mut node);
            node.value()
                .or_else(|| node.label())
                .map(str::to_owned)
                .or_else(|| content_name(elements))
        })
        .collect();
    (!words.is_empty()).then(|| words.join(" "))
}
