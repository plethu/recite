//! Native delivery of uncommitted camera movement to the toolkit-neutral local pan.
use super::*;
pub(crate) use recite_writer_app::map::input::pan::*;
use recite_writer_session::map::camera::Camera;
use std::time::Duration;

impl InputWidget {
    pub fn begin_local_pan(&mut self, point: Point) {
        if matches!(self.kind, Kind::Viewport(_)) {
            self.pan.pending = Pending::Drag { anchor: point };
        }
    }
    /// Translates a viewport drag locally; returns false when the drag is not owned here.
    pub fn drag_local_pan(&mut self, ctx: &mut EventCtx<'_>, point: Point) -> bool {
        let Pending::Drag { anchor } = &mut self.pan.pending else {
            return false;
        };
        let offset = point - *anchor;
        if LocalPan::exposes_unmounted_edge(offset) {
            *anchor = point;
            self.pan.offset = Vec2::ZERO;
            ctx.submit_action::<NativeAction>(Input::Move(point));
        } else {
            self.pan.offset = offset;
        }
        ctx.request_compose();
        true
    }
    /// Commits a drag's displayed offset before its press finishes.
    pub fn release_local_pan(&mut self, ctx: &mut EventCtx<'_>, point: Point) {
        if matches!(self.pan.pending, Pending::Drag { .. }) && self.pan.settle().is_some() {
            ctx.request_compose();
            ctx.submit_action::<NativeAction>(Input::Move(point));
        }
    }
    /// Drops a drag's displayed offset; Cancel restores the committed gesture origin.
    pub fn cancel_local_pan(&mut self, ctx: &mut EventCtx<'_>) {
        if matches!(self.pan.pending, Pending::Drag { .. }) && self.pan.settle().is_some() {
            ctx.request_compose();
        }
    }
    pub fn wheel_local_pan(&mut self, ctx: &mut EventCtx<'_>, delta: Vec2) {
        let offset = self.pan.offset + delta;
        ctx.request_compose();
        if LocalPan::exposes_unmounted_edge(offset) {
            self.pan.settle();
            ctx.submit_action::<NativeAction>(Input::Pan(offset));
            return;
        }
        self.pan.offset = offset;
        if !matches!(self.pan.pending, Pending::Wheel { .. }) {
            ctx.request_anim_frame();
        }
        self.pan.pending = Pending::Wheel {
            idle: Duration::ZERO,
        };
    }
    /// Commits pending wheel movement so following input acts on the displayed camera.
    pub fn commit_wheel_pan(&mut self, ctx: &mut EventCtx<'_>) {
        if matches!(self.pan.pending, Pending::Wheel { .. })
            && let Some(offset) = self.pan.settle()
        {
            ctx.request_compose();
            ctx.submit_action::<NativeAction>(Input::Pan(offset));
        }
    }
    pub fn settle_wheel_pan(&mut self, ctx: &mut UpdateCtx<'_>, interval: u64) {
        let Pending::Wheel { idle } = &mut self.pan.pending else {
            return;
        };
        *idle += Duration::from_nanos(interval);
        if *idle < WHEEL_SETTLE {
            ctx.request_anim_frame();
        } else if let Some(offset) = self.pan.settle() {
            ctx.request_compose();
            ctx.submit_action::<NativeAction>(Input::Pan(offset));
        }
    }
    /// Discards displayed movement superseded by Writer's committed camera or gesture.
    pub fn configure_camera(this: &mut WidgetMut<'_, Self>, camera: Camera, panning: bool) {
        let committed = Some(LocalPan::key(camera));
        let pan = &mut this.widget.pan;
        let moved = std::mem::replace(&mut pan.camera, committed) != committed;
        let abandoned = !panning && matches!(pan.pending, Pending::Drag { .. });
        if (moved && matches!(pan.pending, Pending::Wheel { .. })) || abandoned {
            pan.pending = Pending::None;
        }
        if (moved || abandoned) && std::mem::take(&mut pan.offset) != Vec2::ZERO {
            this.ctx.request_compose();
        }
    }
}
