//! A quiet, keyboard-accessible grip. Commit placement on release, not each frame.
use super::{camera::Camera, placement::Arrangement};
use freya::prelude::*;

#[derive(Clone, PartialEq)]
pub(super) struct Handle {
    pub positions: State<Arrangement>,
    pub scene: String,
    pub card: String,
    pub title: String,
    pub x: f32,
    pub y: f32,
    pub camera: State<Camera>,
    pub visible: bool,
    pub focusable: bool,
}
impl Component for Handle {
    fn render(&self) -> impl IntoElement {
        let id = use_a11y();
        let mut drag = use_state(|| None::<(f64, f64, f32, f32)>);
        let mut positions = self.positions;
        let scene = self.scene.clone();
        let move_scene = scene.clone();
        let release_scene = scene.clone();
        let key = self.card.clone();
        let keyboard_key = key.clone();
        let mut camera = self.camera;
        let (x, y) = (self.x, self.y);
        let title = format!("Move {}", self.title);
        TooltipContainer::new(Tooltip::new_text(format!(
            "{title} · drag or use arrow keys"
        )))
        .child(
            rect()
                .width(Size::px(24.))
                .height(Size::px(24.))
                .a11y_id(id)
                .a11y_focusable(self.focusable)
                .a11y_role(AccessibilityRole::Button)
                .a11y_builder(move |node| node.set_label(title.clone()))
                .cursor(CursorIcon::Grab)
                .on_press(|e: Event<PressEventData>| e.stop_propagation())
                .on_pointer_down(move |event: Event<PointerEventData>| {
                    if event.is_primary() {
                        camera.write().framing = super::camera::Framing::Free;
                        let point = event.global_location();
                        drag.set(Some((point.x, point.y, x, y)));
                        event.stop_propagation();
                        id.request_focus();
                    }
                })
                .on_global_pointer_move(move |event: Event<PointerEventData>| {
                    if let Some((start_x, start_y, initial_x, initial_y)) = *drag.peek() {
                        let point = event.global_location();
                        let scale = camera.peek().zoom;
                        positions.write().move_card(
                            &move_scene,
                            &key,
                            initial_x + (point.x - start_x) as f32 / scale,
                            initial_y + (point.y - start_y) as f32 / scale,
                        );
                    }
                })
                .on_capture_global_pointer_press(move |event: Event<PointerEventData>| {
                    if drag.peek().is_some() {
                        event.stop_propagation();
                        event.prevent_default();
                        drag.set(None);
                        positions.write().persist(&release_scene);
                    }
                })
                .on_key_down(move |event: Event<KeyboardEventData>| {
                    let delta = match event.code {
                        Code::ArrowLeft => (-24., 0.),
                        Code::ArrowRight => (24., 0.),
                        Code::ArrowUp => (0., -24.),
                        Code::ArrowDown => (0., 24.),
                        _ => return,
                    };
                    camera.write().framing = super::camera::Framing::Free;
                    let mut state = positions.write();
                    state.move_card(&scene, &keyboard_key, x + delta.0, y + delta.1);
                    state.persist(&scene);
                    event.stop_propagation();
                    event.prevent_default();
                })
                .child(
                    rect()
                        .opacity(
                            if self.visible || id.is_focused() || drag.read().is_some() {
                                1.
                            } else {
                                0.
                            },
                        )
                        .child(label().text("⠿").font_size(18.)),
                ),
        )
    }
}
