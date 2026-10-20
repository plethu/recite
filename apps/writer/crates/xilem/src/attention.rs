//! Focus overlays the selected workspace; native pane requests carry their own identity.
use crate::{Phase, Writer, map::Workspace};
use std::sync::Arc;
use xilem::masonry::{core, widgets};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PaneMotion {
    Left,
    Right,
    Up,
    Down,
    Next,
    Previous,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Pane {
    Navigation,
    Map,
    Script,
    Preview,
    Declarations,
    Build,
    Settings,
}
#[derive(Clone, Copy, Default, PartialEq)]
pub(crate) struct Region(pub Option<Pane>);
impl core::UsesProperty<Region> for widgets::SizedBox {}
impl core::UsesProperty<Region> for widgets::Flex {}
impl core::Property for Region {
    fn static_default() -> &'static Self {
        static NONE: Region = Region(None);
        &NONE
    }
}
#[derive(Clone, Debug)]
pub(crate) struct PaneRequest(Arc<PaneMotion>);
impl PartialEq for PaneRequest {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl PaneRequest {
    pub fn motion(&self) -> PaneMotion {
        *self.0
    }
}
#[derive(Clone, Default, PartialEq)]
pub(crate) struct Attention {
    focused: bool,
    pane: Option<PaneRequest>,
}
impl Attention {
    pub fn is_focused(&self) -> bool {
        self.focused
    }
    pub fn leave_focus(&mut self) {
        self.focused = false;
    }
    pub fn pane_request(&self) -> Option<&PaneRequest> {
        self.pane.as_ref()
    }
}
impl Writer {
    pub(crate) fn visible_workspace(&self) -> Workspace {
        if self.attention.focused {
            Workspace::Script
        } else {
            self.map.workspace
        }
    }
    pub(crate) fn toggle_focus(&mut self) {
        if self.phase != Phase::Writing || self.work.busy() || self.navigation.busy() {
            self.status = "Finish the current action before changing writing focus.".into();
            return;
        }
        self.cancel_map_gesture();
        self.workspaces.screen = crate::workspaces::Screen::Write;
        self.attention.focused = !self.attention.focused;
        self.status = if self.attention.focused {
            "Writing focus"
        } else {
            "Returned to the workspace."
        }
        .into();
    }
    pub(crate) fn focus_pane(&mut self, direction: PaneMotion) {
        self.attention.pane = Some(PaneRequest(Arc::new(direction)));
    }
}

#[cfg(test)]
mod tests;
