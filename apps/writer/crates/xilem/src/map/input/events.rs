use super::*;
use xilem::masonry::{
    core::{
        PointerButton, PointerEvent, PointerGesture,
        keyboard::{Key, NamedKey},
    },
    dpi::PhysicalPosition,
};

impl InputWidget {
    pub fn pointer(&mut self, ctx: &mut EventCtx<'_>, event: &PointerEvent) {
        if self.kind.is_card() {
            match event {
                PointerEvent::Up(event) => self.clicks = event.state.count,
                PointerEvent::Cancel(_) | PointerEvent::Down(_) => self.clicks = 0,
                _ => {}
            }
            return;
        }
        self.retire_camera_press(ctx, event);
        if ctx.is_handled() {
            return;
        }
        match event {
            PointerEvent::Down(event)
                if matches!(event.button, None | Some(PointerButton::Primary))
                    && ctx.pointer_capture_target_id().is_none() =>
            {
                ctx.capture_pointer();
                ctx.request_focus();
                self.commit_wheel_pan(ctx);
                let point = event.state.logical_position();
                let point = Point::new(point.x, point.y);
                self.press = Some(PointerPress {
                    button: event.button,
                    motion: Motion::ClickCandidate(point),
                });
                self.begin_local_pan(point);
                ctx.submit_action::<NativeAction>(Input::Begin(point));
                ctx.set_handled();
            }
            PointerEvent::Move(event) if self.press.is_some() => {
                let point = event.current.logical_position();
                let point = Point::new(point.x, point.y);
                self.track_motion(point);
                if !self.drag_local_pan(ctx, point) {
                    ctx.submit_action::<NativeAction>(Input::Move(point));
                }
                ctx.set_handled();
            }
            PointerEvent::Up(event)
                if self.press.is_some_and(|press| press.button == event.button) =>
            {
                let position = event.state.logical_position();
                self.release_local_pan(ctx, Point::new(position.x, position.y));
                self.finish_press(
                    ctx,
                    Point::new(position.x, position.y),
                    event.state.position,
                );
                ctx.set_handled();
            }
            PointerEvent::Cancel(_) if self.press.is_some() => {
                self.press = None;
                self.cancel_local_pan(ctx);
                ctx.submit_action::<NativeAction>(Input::Cancel);
                ctx.set_handled();
            }
            PointerEvent::Scroll(event) if matches!(self.kind, Kind::Viewport(_)) => {
                let scale = ctx.scale_factor();
                let delta = event
                    .delta
                    .to_pixel_delta(
                        PhysicalPosition::new(60. * scale, 60. * scale),
                        PhysicalPosition::new(
                            ctx.content_box().width() * scale,
                            ctx.content_box().height() * scale,
                        ),
                    )
                    .to_logical::<f64>(scale);
                if event.state.modifiers.ctrl() || event.state.modifiers.meta() {
                    self.commit_wheel_pan(ctx);
                    ctx.submit_action::<NativeAction>(Input::Zoom {
                        factor: (-delta.y as f32 / 300.).exp(),
                        anchor: ctx.local_position(event.state.position),
                    });
                } else {
                    self.wheel_local_pan(ctx, Vec2::new(delta.x, delta.y));
                }
                ctx.set_handled();
            }
            PointerEvent::Gesture(event) if matches!(self.kind, Kind::Viewport(_)) => {
                if let PointerGesture::Pinch(delta) = event.gesture {
                    self.commit_wheel_pan(ctx);
                    ctx.submit_action::<NativeAction>(Input::Zoom {
                        factor: (1. + delta).max(0.001),
                        anchor: ctx.local_position(event.state.position),
                    });
                    ctx.set_handled();
                }
            }
            _ => {}
        }
    }
    pub fn keyboard(&mut self, ctx: &mut EventCtx<'_>, event: &TextEvent) {
        if self.kind.is_card() || ctx.is_handled() {
            return;
        }
        let TextEvent::Keyboard(event) = event else {
            return;
        };
        if !event.state.is_down() || !event.modifiers.is_empty() {
            return;
        }
        if event.key == Key::Named(NamedKey::Escape) && self.press.is_some() {
            self.press = None;
            self.cancel_local_pan(ctx);
            ctx.release_pointer();
            ctx.submit_action::<NativeAction>(Input::Cancel);
            ctx.set_handled();
            return;
        }
        let direction = match &event.key {
            Key::Named(NamedKey::ArrowLeft) => Some((-1., 0.)),
            Key::Named(NamedKey::ArrowRight) => Some((1., 0.)),
            Key::Named(NamedKey::ArrowUp) => Some((0., -1.)),
            Key::Named(NamedKey::ArrowDown) => Some((0., 1.)),
            Key::Character(key) if self.vim => match key.as_str() {
                "h" => Some((-1., 0.)),
                "l" => Some((1., 0.)),
                "k" => Some((0., -1.)),
                "j" => Some((0., 1.)),
                _ => None,
            },
            _ => None,
        };
        let action = if let Some((x, y)) = direction {
            Some(Input::Direction(Vec2::new(x, y)))
        } else if ctx.is_focus_target()
            && (event.key == Key::Named(NamedKey::Enter)
                || (self.vim && event.key == Key::Character("i".into())))
        {
            Some(Input::Open)
        } else if event.key == Key::Character("/".into()) {
            Some(Input::Search)
        } else {
            None
        };
        if let Some(action) = action {
            self.commit_wheel_pan(ctx);
            ctx.submit_action::<NativeAction>(action);
            ctx.set_handled();
        }
    }
}
