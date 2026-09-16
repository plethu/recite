//! Compact, named controls shared by the writer's contextual toolbars.
use crate::palette;
use freya::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Icon {
    Sidebar,
    Settings,
    Undo,
    Redo,
    ZoomIn,
    ZoomOut,
}

impl Icon {
    fn render(self) -> Element {
        let shape = match self {
            Self::Settings => {
                "<circle cx='12' cy='12' r='3'/><path d='M9 3h6l1 3 3 1 2 5-2 5-3 1-1 3H9l-1-3-3-1-2-5 2-5 3-1Z'/>"
            }
            Self::ZoomIn => "<path d='M5 12h14M12 5v14'/>",
            Self::ZoomOut => "<path d='M5 12h14'/>",
            Self::Sidebar => "<rect x='3' y='4' width='18' height='16' rx='2'/><path d='M9 4v16'/>",
            Self::Undo => "<path d='M8 5L3 10l5 5M3 10h11a6 6 0 0 1 0 12'/>",
            Self::Redo => "<path d='M16 5l5 5-5 5M21 10H10a6 6 0 0 0 0 12'/>",
        };
        let svg = format!(
            "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='1.6' stroke-linecap='round' stroke-linejoin='round'>{shape}</svg>"
        );
        SvgViewer::new(Bytes::from(svg.into_bytes()))
            .width(Size::px(20.))
            .height(Size::px(20.))
            .stroke(use_theme().read().colors.text_primary)
            .into_element()
    }
}

#[derive(Clone, PartialEq)]
pub(super) struct IconButton {
    name: String,
    icon: Icon,
    action: EventHandler<()>,
}

impl IconButton {
    pub fn new(name: impl Into<String>, icon: Icon, action: impl FnMut() + 'static) -> Self {
        let mut action = action;
        Self {
            name: name.into(),
            icon,
            action: EventHandler::new(move |()| action()),
        }
    }
}

impl Component for IconButton {
    fn render(&self) -> impl IntoElement {
        let id = use_a11y();
        let action = self.action.clone();
        TooltipContainer::new(Tooltip::new_text(self.name.clone())).child(
            rect().a11y_id(id).a11y_role(AccessibilityRole::Button).a11y_alt(self.name.clone())
                .a11y_focusable(true).cursor(CursorIcon::Pointer)
                .width(Size::px(32.)).height(Size::px(32.))
                .main_align(Alignment::Center).cross_align(Alignment::Center)
                .border(Border::new().width(if id.is_focused() { 2. } else { 0. }).fill((120, 140, 110)))
                .on_all_press(move |event: Event<PressEventData>| {
                    if matches!(event.data(), PressEventData::Mouse(data) if data.button != Some(MouseButton::Left)) { return; }
                    id.request_focus(); action.call(());
                })
                .child(self.icon.render()),
        )
    }
}

pub(super) fn navigation_row(
    text: String,
    selected: bool,
    dark: bool,
    action: impl FnMut(Event<PressEventData>) + 'static,
) -> Element {
    NavigationRow {
        text,
        selected,
        dark,
        action: EventHandler::new(action),
    }
    .into_element()
}

#[derive(Clone, PartialEq)]
struct NavigationRow {
    text: String,
    selected: bool,
    dark: bool,
    action: EventHandler<Event<PressEventData>>,
}
impl Component for NavigationRow {
    fn render(&self) -> impl IntoElement {
        let id = use_a11y();
        let text = self.text.clone();
        let selected = self.selected;
        let dark = self.dark;
        let action = self.action.clone();
        rect()
            .width(Size::fill())
            .padding((5., 6.))
            .a11y_id(id)
            .a11y_focusable(true)
            .a11y_role(AccessibilityRole::Button)
            .a11y_alt(text.clone())
            .a11y_builder(move |node| {
                node.set_toggled(if selected {
                    accesskit::Toggled::True
                } else {
                    accesskit::Toggled::False
                })
            })
            .background(if selected {
                palette::selection(dark)
            } else {
                Color::TRANSPARENT
            })
            .border(
                Border::new()
                    .width(if id.is_focused() { 1. } else { 0. })
                    .fill(palette::accent(dark)),
            )
            .cursor(CursorIcon::Pointer)
            .on_all_press(move |event: Event<PressEventData>| {
                id.request_focus();
                action.call(event);
            })
            .child(label().text(text).text_align(TextAlign::Left))
            .into_element()
    }
}
