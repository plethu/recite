//! Dialog geometry never scales text; focus order belongs to the dialog contents.
use super::tokens as t;
use freya::{
    animation::{OnCreation, use_animation},
    prelude::*,
};

#[derive(Clone, PartialEq)]
pub(crate) struct Dialog {
    pub title: String,
    pub content: Element,
    pub actions: Element,
    pub primary: super::SubmitAction,
    pub focus_order: Vec<AccessibilityId>,
    pub close: EventHandler<()>,
    pub reduced_motion: bool,
    pub dismissal_only: bool,
}
impl Component for Dialog {
    fn render(&self) -> impl IntoElement {
        let colors = t::colors();
        let reduced = self.reduced_motion;
        let ink = use_animation(move |config| {
            config.on_creation(OnCreation::Run);
            t::transition(0., 1., reduced)
        });
        let close = self.close.clone();
        let mut order = self.focus_order.clone();
        if !self.primary.enabled {
            order.retain(|id| *id != self.primary.id);
        }
        let primary = self.primary.clone();
        let primary_button = if self.dismissal_only {
            let action = self.primary.clone();
            super::Button::new()
                .flat()
                .a11y_id(action.id)
                .enabled(action.enabled)
                .named(action.caption.clone())
                .child(action.caption.clone())
                .on_press(move |_| action.run())
                .into_element()
        } else {
            self.primary.button().into_element()
        };
        rect()
            .position(Position::new_global().top(0.).left(0.))
            .width(Size::window_percent(100.))
            .height(Size::window_percent(100.))
            .layer(100)
            .center()
            .background(Color::from_argb(110, 0, 0, 0))
            .on_pointer_down(|e: Event<PointerEventData>| e.stop_propagation())
            .on_all_press(|e: Event<PressEventData>| e.stop_propagation())
            .on_global_key_down(move |e: Event<KeyboardEventData>| {
                if super::keyboard::submit_key(&e) {
                    e.stop_propagation();
                    e.prevent_default();
                    primary.run();
                } else if e.key == Key::Named(NamedKey::Escape) {
                    e.stop_propagation();
                    e.prevent_default();
                    close.call(());
                } else if e.key == Key::Named(NamedKey::Tab) && !order.is_empty() {
                    let current = *Platform::get().focused_accessibility_id.peek();
                    let index = order.iter().position(|id| *id == current).unwrap_or(0);
                    let step = if e.modifiers.contains(Modifiers::SHIFT) {
                        order.len() - 1
                    } else {
                        1
                    };
                    order[(index + step) % order.len()].request_focus();
                    e.stop_propagation();
                    e.prevent_default();
                }
            })
            .child(
                rect()
                    .width(Size::window_percent(90.))
                    .max_width(Size::px(620. * t::ui_scale()))
                    .max_height(Size::window_percent(90.))
                    .padding(t::SPACE_XL)
                    .spacing(t::SPACE_XL)
                    .corner_radius(t::DIALOG_RADIUS)
                    .background(colors.floating)
                    .shadow((0., 12., 36., 0., colors.shadow))
                    .color(colors.ink)
                    .border(Border::new().width(1.).fill(colors.rule))
                    .a11y_role(AccessibilityRole::Dialog)
                    .a11y_alt(self.title.clone())
                    .opacity(ink.get().value())
                    .child(label().text(self.title.clone()).font_size(t::title()))
                    .child(
                        ScrollView::new()
                            .height(Size::auto())
                            .width(Size::fill())
                            .max_height(Size::window_percent(60.))
                            .child(self.content.clone()),
                    )
                    .child(
                        actions()
                            .content(Content::Flex)
                            .child(rect().width(Size::flex(1.)).child(self.actions.clone()))
                            .child(primary_button),
                    ),
            )
    }
}

pub(crate) fn actions() -> Rect {
    rect()
        .horizontal()
        .width(Size::fill())
        .spacing(t::SPACE_SM)
        .main_align(Alignment::End)
        .cross_align(Alignment::Center)
}

#[cfg(test)]
mod tests;
