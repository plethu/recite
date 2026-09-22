//! Compact, named controls shared by the writer's contextual toolbars.
use freya::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Icon {
    Back,
    Link,
    Forward,
    Close,
    Confirm,
    Sidebar,
    Settings,
    Undo,
    Redo,
    ZoomIn,
    ZoomOut,
    Search,
    ChevronDown,
}

impl Icon {
    pub(crate) fn render(self) -> Element {
        self.into_element()
    }
    pub(crate) fn colored(self, color: Color) -> Element {
        let shape = match self {
            Self::Search => "<circle cx='10.5' cy='10.5' r='6.5'/><path d='m16 16 5 5'/>",
            Self::ChevronDown => "<path d='m6 9 6 6 6-6'/>",
            Self::Link => "<path d='M10 13l4-2M9 16H7a4 4 0 0 1 0-8h3M15 8h2a4 4 0 0 1 0 8h-3'/>",
            Self::Back => "<path d='M15 5l-7 7 7 7'/>",
            Self::Forward => "<path d='M9 5l7 7-7 7'/>",
            Self::Close => "<path d='M6 6l12 12M18 6L6 18'/>",
            Self::Confirm => "<path d='M5 12l4 4L19 6'/>",
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
            .width(Size::px(crate::design::tokens::ICON_SIZE))
            .height(Size::px(crate::design::tokens::ICON_SIZE))
            .stroke(color)
            .into_element()
    }
}

#[derive(Clone, PartialEq)]
pub(super) struct IconButton {
    name: String,
    icon: Icon,
    action: EventHandler<()>,
    enabled: bool,
}

impl IconButton {
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    pub fn new(name: impl Into<String>, icon: Icon, action: impl FnMut() + 'static) -> Self {
        let mut action = action;
        Self {
            name: name.into(),
            icon,
            action: EventHandler::new(move |()| action()),
            enabled: true,
        }
    }
}

impl Component for IconButton {
    fn render(&self) -> impl IntoElement {
        let action = self.action.clone();
        TooltipContainer::new(Tooltip::new_text(self.name.clone())).child(
            crate::design::Button::new()
                .flat()
                .enabled(self.enabled)
                .named(self.name.clone())
                .width(Size::px(crate::design::tokens::control_height()))
                .on_press(move |_| action.call(()))
                .child(self.icon.colored(if self.enabled {
                    crate::design::tokens::colors().ink
                } else {
                    crate::design::tokens::colors().muted
                })),
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
        crate::design::Button::new()
            .flat()
            .named(self.text.clone())
            .selected(self.selected)
            .width(Size::fill())
            .on_press(self.action.clone())
            .child(
                label()
                    .text(self.text.clone())
                    .width(Size::fill())
                    .text_align(TextAlign::Left),
            )
            .into_element()
    }
}

impl Component for Icon {
    fn render(&self) -> impl IntoElement {
        self.colored(crate::design::tokens::colors().ink)
    }
}
