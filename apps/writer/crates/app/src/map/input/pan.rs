//! Uncommitted camera movement translates retained viewport content without a rebuild.
//!
//! Mounted cards, routes and the dot grid already extend `OVERSCAN` beyond the window, so the
//! viewport can translate them locally until that margin would be exposed. Crossing it,
//! releasing a drag or letting wheel scrolling settle commits through the ordinary camera
//! input, which remounts content around the committed camera.
use kurbo::{Point, Vec2};
use recite_writer_session::map::{camera::Camera, topology::OVERSCAN};
use std::time::Duration;

/// Idle time after the last wheel delta before its pending movement is committed.
pub const WHEEL_SETTLE: Duration = Duration::from_millis(120);

/// The input that owns the displayed, uncommitted offset. A drag commits pending wheel
/// movement before it begins, and wheel input supersedes a drag.
#[derive(Default)]
pub enum Pending {
    #[default]
    None,
    /// The window point represented by the committed camera.
    Drag { anchor: Point },
    /// Time since the latest wheel delta.
    Wheel { idle: Duration },
}

#[derive(Default)]
pub struct LocalPan {
    pub offset: Vec2,
    pub pending: Pending,
    /// Committed camera the offset is relative to.
    pub camera: Option<(f32, f32, f32)>,
}
impl LocalPan {
    pub fn committed(camera: Camera) -> Self {
        Self {
            camera: Some(Self::key(camera)),
            ..Self::default()
        }
    }
    pub fn key(camera: Camera) -> (f32, f32, f32) {
        (camera.x, camera.y, camera.zoom)
    }
    pub fn exposes_unmounted_edge(offset: Vec2) -> bool {
        let limit = f64::from(OVERSCAN);
        offset.x.abs() > limit || offset.y.abs() > limit
    }
    pub fn offset(&self) -> Vec2 {
        self.offset
    }
    /// Ends the pending input, returning a nonzero offset still to commit.
    pub fn settle(&mut self) -> Option<Vec2> {
        self.pending = Pending::None;
        let offset = std::mem::take(&mut self.offset);
        (offset != Vec2::ZERO).then_some(offset)
    }
}
