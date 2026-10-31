//! Uncommitted camera movement translates retained viewport content without a rebuild.
//!
//! Mounted cards, routes and the dot grid already extend `OVERSCAN` beyond the window, so the
//! viewport can translate them locally until that margin would be exposed. Crossing it,
//! releasing a drag or letting wheel scrolling settle commits through the ordinary camera
//! input, which remounts content around the committed camera.
use super::*;
use recite_writer_session::map::{camera::Camera, topology::OVERSCAN};
use std::time::Duration;

/// Idle time after the last wheel delta before its pending movement is committed.
const WHEEL_SETTLE: Duration = Duration::from_millis(120);

/// The input that owns the displayed, uncommitted offset. A drag commits pending wheel
/// movement before it begins, and wheel input supersedes a drag.
#[derive(Default)]
enum Pending {
    #[default]
    None,
    /// The window point represented by the committed camera.
    Drag { anchor: Point },
    /// Time since the latest wheel delta.
    Wheel { idle: Duration },
}

#[derive(Default)]
pub(super) struct LocalPan {
    offset: Vec2,
    pending: Pending,
    /// Committed camera the offset is relative to.
    camera: Option<(f32, f32, f32)>,
}
impl LocalPan {
    pub(super) fn committed(camera: Camera) -> Self {
        Self {
            camera: Some(Self::key(camera)),
            ..Self::default()
        }
    }
    fn key(camera: Camera) -> (f32, f32, f32) {
        (camera.x, camera.y, camera.zoom)
    }
    fn exposes_unmounted_edge(offset: Vec2) -> bool {
        let limit = f64::from(OVERSCAN);
        offset.x.abs() > limit || offset.y.abs() > limit
    }
    pub(super) fn offset(&self) -> Vec2 {
        self.offset
    }
    /// Ends the pending input, returning a nonzero offset still to commit.
    fn settle(&mut self) -> Option<Vec2> {
        self.pending = Pending::None;
        let offset = std::mem::take(&mut self.offset);
        (offset != Vec2::ZERO).then_some(offset)
    }
}

impl InputWidget {
    pub(super) fn begin_local_pan(&mut self, point: Point) {
        if matches!(self.kind, Kind::Viewport(_)) {
            self.pan.pending = Pending::Drag { anchor: point };
        }
    }
    /// Translates a viewport drag locally; returns false when the drag is not owned here.
    pub(super) fn drag_local_pan(&mut self, ctx: &mut EventCtx<'_>, point: Point) -> bool {
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
    pub(super) fn release_local_pan(&mut self, ctx: &mut EventCtx<'_>, point: Point) {
        if matches!(self.pan.pending, Pending::Drag { .. }) && self.pan.settle().is_some() {
            ctx.request_compose();
            ctx.submit_action::<NativeAction>(Input::Move(point));
        }
    }
    /// Drops a drag's displayed offset; Cancel restores the committed gesture origin.
    pub(super) fn cancel_local_pan(&mut self, ctx: &mut EventCtx<'_>) {
        if matches!(self.pan.pending, Pending::Drag { .. }) && self.pan.settle().is_some() {
            ctx.request_compose();
        }
    }
    pub(super) fn wheel_local_pan(&mut self, ctx: &mut EventCtx<'_>, delta: Vec2) {
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
    pub(super) fn commit_wheel_pan(&mut self, ctx: &mut EventCtx<'_>) {
        if matches!(self.pan.pending, Pending::Wheel { .. })
            && let Some(offset) = self.pan.settle()
        {
            ctx.request_compose();
            ctx.submit_action::<NativeAction>(Input::Pan(offset));
        }
    }
    pub(super) fn settle_wheel_pan(&mut self, ctx: &mut UpdateCtx<'_>, interval: u64) {
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
    pub(super) fn configure_camera(this: &mut WidgetMut<'_, Self>, camera: Camera, panning: bool) {
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
