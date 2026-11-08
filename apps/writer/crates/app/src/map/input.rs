//! Toolkit-neutral input the Map viewport delivers to authoring state.
pub use super::Detail;
use kurbo::{Point, Vec2};
use std::sync::Arc;
pub mod pan;
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Viewport(Detail),
    Grip(String),
    Card { title: String, selected: bool },
}
impl Kind {
    pub fn is_card(&self) -> bool {
        matches!(self, Self::Card { .. })
    }
}
#[derive(Debug)]
pub enum Input {
    Begin(Point),
    Move(Point),
    Finish,
    OverviewClick(Point),
    Cancel,
    Pan(Vec2),
    Zoom { factor: f32, anchor: Point },
    Direction(Vec2),
    Open,
    Search,
    Focus,
    Hover(bool),
    Focused(FocusToken),
}

#[derive(Clone, Debug)]
pub struct FocusToken(Arc<()>);
impl FocusToken {
    pub fn new() -> Self {
        Self(Arc::new(()))
    }
}
impl PartialEq for FocusToken {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct FocusRequest {
    pub token: FocusToken,
    /// Whole-card bounds in the untransformed viewport's local coordinates.
    pub bounds: kurbo::Rect,
}

impl Default for FocusToken {
    fn default() -> Self {
        Self::new()
    }
}
