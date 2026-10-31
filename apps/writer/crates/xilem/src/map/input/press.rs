//! Native press ownership ends when another gesture takes over.
use super::*;
use xilem::masonry::{
    core::{PointerEvent, PointerGesture},
    dpi::PhysicalPosition,
};

impl InputWidget {
    pub(super) fn retire_camera_press(&mut self, ctx: &mut EventCtx<'_>, event: &PointerEvent) {
        let superseded = matches!(event, PointerEvent::Scroll(_))
            || matches!(event, PointerEvent::Gesture(event) if matches!(event.gesture, PointerGesture::Pinch(_)));
        if superseded && self.press.take().is_some() {
            self.cancel_local_pan(ctx);
            if ctx.is_pointer_capture_target() {
                ctx.release_pointer();
            }
            ctx.submit_action::<NativeAction>(Input::Cancel);
        }
    }
    pub(super) fn track_motion(&mut self, point: Point) {
        if let Some(press) = &mut self.press
            && let Motion::ClickCandidate(start) = press.motion
            && (point - start).hypot() > 4.
        {
            press.motion = Motion::Dragging;
        }
    }
    pub(super) fn finish_press(
        &mut self,
        ctx: &mut EventCtx<'_>,
        window: Point,
        physical: PhysicalPosition<f64>,
    ) {
        let press = self.press.take();
        ctx.submit_action::<NativeAction>(Input::Finish);
        if self.kind == Kind::Viewport(Detail::Overview)
            && let Some(PointerPress {
                motion: Motion::ClickCandidate(start),
                ..
            }) = press
            && (window - start).hypot() <= 4.
        {
            ctx.submit_action::<NativeAction>(Input::OverviewClick(ctx.local_position(physical)));
        }
    }
}
