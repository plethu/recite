//! Where an inline rename begins: the heading, a sidebar row or a map card, and where focus
//! returns once it ends.
use super::*;
use crate::map::Workspace;

/// Where a beat's name becomes a field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// The selected beat's heading in Script.
    Heading,
    /// The beat's row in the sidebar outline.
    Outline,
    /// The beat's card on the map.
    Card,
}
/// A beat and the place its name is edited, as F2 or a double-click on a row or card asks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub beat: String,
    pub place: Place,
}

impl Writer {
    /// Where a finished inline rename returns focus: the row or card it was typed in, under the
    /// beat's current name.
    pub fn rename_return(&self) -> Option<&Target> {
        self.workspaces.rename.returning.as_ref()
    }
    /// Rename beat, from F2 or the heading's action: in place on the map card of the selected
    /// beat in Map and on the selected beat's heading in Script and Split; with no beat to rename
    /// there, a picker chooses one first, as the Commands entry always does.
    pub fn rename_beat(&mut self) {
        match self.default_rename_target() {
            Some(target) => self.rename_beat_in(target),
            None => self.choose_beat_to_rename(),
        }
    }
    fn default_rename_target(&self) -> Option<Target> {
        if self.workspaces.screen != Screen::Write || self.session.project().is_none() {
            return None;
        }
        if self.visible_workspace() == Workspace::Map {
            return self.map_selected().map(|beat| Target {
                beat: beat.to_owned(),
                place: Place::Card,
            });
        }
        let beat = self.session.model().selected_block().ok().flatten()?;
        Some(Target {
            beat,
            place: Place::Heading,
        })
    }
    /// Turn the beat's name at `target` into a field holding its id; F2 or a double-click on a
    /// sidebar row or map card, and the heading's Rename beat action, begin here.
    pub fn rename_beat_in(&mut self, target: Target) {
        if self.workspaces.screen != Screen::Write
            || self.session.project().is_none()
            || self.project_action_blocked()
            || self.inline_rename().is_some()
            || !self.shows_rename_place(&target)
        {
            if target.place == Place::Outline && self.navigation_width() <= 0. {
                self.status = "Show the scene list (Ctrl+Shift+B) to rename a beat there.".into();
            }
            return;
        }
        self.workspaces.rename.returning = None;
        // A new rename makes the last one's outcome stale, such as a discarded review.
        self.status.clear();
        self.workspaces.rename.inline = Some(Inline {
            name: target.beat.clone(),
            beat: target.beat,
            stage: Stage::Editing,
            place: target.place,
            anchor: None,
        });
        if target.place == Place::Heading {
            self.focus_pane(PaneMotion::To(Pane::Script));
        }
    }
    /// The heading is the selected beat's; a row or card names a beat of the open scene, in the
    /// sidebar or on the map while they show.
    fn shows_rename_place(&self, target: &Target) -> bool {
        let model = self.session.model();
        match target.place {
            Place::Heading => model.selected_block().ok().flatten().as_ref() == Some(&target.beat),
            Place::Outline => {
                self.navigation_width() > 0.
                    && model
                        .document()
                        .block_summaries()
                        .iter()
                        .any(|block| block.id().as_str() == target.beat)
            }
            Place::Card => {
                self.visible_workspace().has_map()
                    && self
                        .map
                        .scene(self)
                        .is_some_and(|scene| scene.projection.graph.ids.contains_key(&target.beat))
            }
        }
    }
}
