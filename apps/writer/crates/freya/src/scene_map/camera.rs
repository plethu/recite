//! One world-to-viewport transform, shared by rendering and drag coordinates.
use freya::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Framing {
    Entry,
    Fit,
    Free,
}

#[derive(Clone, Copy, PartialEq)]
pub(super) struct Camera {
    pub zoom: f32,
    pub x: f32,
    pub y: f32,
    pub framing: Framing,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            zoom: 1.,
            x: 0.,
            y: 0.,
            framing: Framing::Entry,
        }
    }
}
impl Camera {
    pub fn frame_entry(&mut self, viewport: (f32, f32), entry: (f32, f32, f32)) {
        self.zoom = 1.;
        self.x = (viewport.0 - entry.2) / 2. - entry.0;
        self.y = 32. - entry.1;
        self.framing = Framing::Entry;
    }

    pub fn fit(&mut self, viewport: (f32, f32), bounds: (f32, f32, f32, f32)) {
        let (left, top, width, height) = bounds;
        self.zoom = ((viewport.0 - 32.) / width)
            .min((viewport.1 - 32.) / height)
            .clamp(0.001, 1.);
        self.x = (viewport.0 - width * self.zoom) / 2. - left * self.zoom;
        self.y = 16. - top * self.zoom;
        self.framing = Framing::Fit;
    }
    pub fn zoom_to(&mut self, zoom: f32, viewport: (f32, f32)) {
        self.zoom_at(zoom, (viewport.0 / 2., viewport.1 / 2.));
    }
    pub fn zoom_at(&mut self, zoom: f32, anchor: (f32, f32)) {
        let zoom = zoom.clamp(0.001, 2.);
        let ratio = zoom / self.zoom;
        self.x = anchor.0 - (anchor.0 - self.x) * ratio;
        self.y = anchor.1 - (anchor.1 - self.y) * ratio;
        self.zoom = zoom;
        self.framing = Framing::Free;
    }
}
pub(super) fn viewport(
    mut camera: State<Camera>,
    mut size: State<(f32, f32)>,
    mut drag: State<Option<(f64, f64)>>,
    canvas: Element,
    writer: crate::editing::Writer,
    blocks: Vec<recite_writer_model::ScriptBlock>,
    nodes: Vec<super::layout::Node>,
) -> Element {
    let id = writer.map_focus;
    let selected = writer.selection.read().clone().unwrap_or_default();
    let name = format!(
        "Scene map. Selected {}. Arrow keys select beats. Enter edits. Tab reaches connections. Slash finds a beat.",
        crate::palette::display_name(&selected)
    );
    let view = *camera.read();
    let mut control = use_state(|| false);
    rect()
        .key("map-viewport")
        .a11y_id(id)
        .a11y_focusable(true)
        .a11y_role(AccessibilityRole::ScrollView)
        .a11y_alt(name)
        .width(Size::fill())
        .height(Size::flex(1.))
        .overflow(Overflow::Clip)
        .on_sized(move |e: Event<SizedEventData>| {
            size.set_if_modified((e.area.width(), e.area.height()));
        })
        .on_pointer_down(move |e: Event<PointerEventData>| {
            if e.is_primary() {
                id.request_focus();
                drag.set(Some((e.global_location().x, e.global_location().y)));
                e.stop_propagation();
            }
        })
        .on_global_pointer_move(move |e: Event<PointerEventData>| {
            let previous = *drag.peek();
            if let Some((x, y)) = previous {
                let p = e.global_location();
                let mut c = camera.write();
                c.x += (p.x - x) as f32;
                c.y += (p.y - y) as f32;
                c.framing = Framing::Free;
                drag.set(Some((p.x, p.y)));
            }
        })
        .on_capture_global_pointer_press(move |event: Event<PointerEventData>| {
            if drag.peek().is_some() {
                event.stop_propagation();
                event.prevent_default();
                drag.set(None);
            }
        })
        .on_global_key_down(move |e: Event<KeyboardEventData>| {
            control.set_if_modified(e.modifiers.contains(Modifiers::CONTROL));
        })
        .on_global_key_up(move |e: Event<KeyboardEventData>| {
            control.set_if_modified(e.modifiers.contains(Modifiers::CONTROL));
        })
        .on_key_down(move |e| super::navigation::key(e, writer, &blocks, &nodes))
        .on_wheel(move |e: Event<WheelEventData>| {
            let mut c = camera.write();
            if *control.peek() {
                let anchor = if writer.preferences.peek().config.writer.zoom_to_pointer {
                    (e.element_location.x as f32, e.element_location.y as f32)
                } else {
                    (size.peek().0 / 2., size.peek().1 / 2.)
                };
                let zoom = c.zoom * (e.delta_y as f32 * 0.005).exp();
                c.zoom_at(zoom, anchor);
            } else {
                c.x += e.delta_x as f32;
                c.y += e.delta_y as f32;
                c.framing = Framing::Free;
            }
            e.stop_propagation();
        })
        .child(
            rect()
                .position(Position::new_absolute().left(view.x).top(view.y))
                .child(canvas),
        )
        .into_element()
}
#[cfg(test)]
mod tests;
