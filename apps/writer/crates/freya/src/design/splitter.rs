//! A bounded divider supports pointer dragging and keyboard resizing.
use super::tokens as t;
use freya::prelude::*;

#[derive(Clone, PartialEq)]
pub(crate) struct Splitter {
    pub name: &'static str,
    pub width: State<f32>,
    pub min: f32,
    pub max: f32,
    pub direction: f32,
}
impl Component for Splitter {
    fn render(&self) -> impl IntoElement {
        let colors = t::colors();
        let id = use_a11y();
        let mut drag = use_state(|| None::<(f64, f32, f32)>);
        let mut width = self.width;
        let (min, max, direction) = (self.min, self.max.max(self.min), self.direction);
        rect()
            .width(Size::px(t::SPLITTER_WIDTH))
            .height(Size::fill())
            .cursor(CursorIcon::ColResize)
            .a11y_id(id)
            .a11y_focusable(true)
            .a11y_role(AccessibilityRole::Splitter)
            .a11y_alt(self.name)
            .a11y_builder(move |node| {
                node.set_numeric_value(f64::from(width.read().clamp(min, max)));
                node.set_min_numeric_value(f64::from(min));
                node.set_max_numeric_value(f64::from(max));
            })
            .background(if id.is_focused() || drag.read().is_some() {
                colors.hover
            } else {
                Color::TRANSPARENT
            })
            .on_pointer_down(move |e: Event<PointerEventData>| {
                if e.is_primary() {
                    id.request_focus();
                    let initial = width.peek().clamp(min, max);
                    drag.set(Some((e.global_location().x, initial, initial)));
                    e.stop_propagation();
                }
            })
            .on_global_pointer_move(move |e: Event<PointerEventData>| {
                let current = *drag.peek();
                if let Some((x, initial, _)) = current {
                    let proposed =
                        (initial + (e.global_location().x - x) as f32 * direction).clamp(min, max);
                    drag.set_if_modified(Some((x, initial, proposed)));
                }
            })
            .on_capture_global_pointer_press(move |e: Event<PointerEventData>| {
                let current = *drag.peek();
                if let Some((_, _, proposed)) = current {
                    width.set_if_modified(proposed);
                    drag.set(None);
                    e.stop_propagation();
                    e.prevent_default();
                }
            })
            .on_key_down(move |e: Event<KeyboardEventData>| {
                if e.code == Code::Escape && drag.peek().is_some() {
                    drag.set(None);
                    e.stop_propagation();
                    e.prevent_default();
                    return;
                }
                let current = width.peek().clamp(min, max);
                let next = match e.code {
                    Code::ArrowLeft => current - t::RESIZE_STEP * direction,
                    Code::ArrowRight => current + t::RESIZE_STEP * direction,
                    Code::Home => min,
                    Code::End => max,
                    _ => return,
                };
                width.set_if_modified(next.clamp(min, max));
                e.stop_propagation();
                e.prevent_default();
            })
            .maybe_child((*drag.read()).map(|(_, initial, proposed)| {
                rect()
                    .position(Position::new_absolute().left((proposed - initial) * direction))
                    .layer(10)
                    .width(Size::px(t::FOCUS_WIDTH))
                    .height(Size::fill())
                    .background(colors.accent)
            }))
            .child(
                rect()
                    .position(Position::new_absolute().left(2.))
                    .width(Size::px(1.))
                    .height(Size::fill())
                    .background(colors.rule),
            )
    }
}
