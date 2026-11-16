//! Renaming a beat in place: its heading in Script, its row in the sidebar or its card on the
//! map becomes a field, Enter reviews the project rename, and a slip under the field says what it
//! changes before it is staged as one Undo step. A rename the review refuses opens the full rename
//! page with its report.
use super::*;
use crate::attention::{Pane, PaneMotion};
use crate::workspaces::{Open, Screen};
mod consequence;
mod place;
pub use consequence::Consequence;
pub use place::{Place, Target};

/// An inline rename of one beat, from the field to the staged change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inline {
    /// The beat's id when the rename began.
    pub beat: String,
    /// The new id as typed.
    pub name: String,
    pub stage: Stage,
    pub place: Place,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// The name is a field.
    Editing,
    /// The project rename is being reviewed.
    Reviewing,
    /// The review is ready: the consequence slip asks to rename or cancel.
    Reviewed,
    /// The reviewed rename is being staged.
    Applying,
}

impl Writer {
    pub fn inline_rename(&self) -> Option<&Inline> {
        self.workspaces.rename.inline.as_ref()
    }
    pub fn edit_inline_rename(&mut self, name: String) {
        if let Some(inline) = &mut self.workspaces.rename.inline
            && inline.stage == Stage::Editing
        {
            inline.name = name;
        }
    }
    /// Enter: review the rename across the project; an unchanged or empty name just ends it.
    pub fn commit_inline_rename(&mut self) {
        let Some(inline) = self.workspaces.rename.inline.as_mut() else {
            return;
        };
        if inline.stage != Stage::Editing {
            return;
        }
        let name = inline.name.trim().to_owned();
        if name.is_empty() || name == inline.beat {
            self.cancel_inline_rename();
            return;
        }
        inline.stage = Stage::Reviewing;
        let beat = inline.beat.clone();
        self.change_rename_block(beat);
        self.change_rename_name(name);
        self.review_project_rename();
        if !self.project_jobs_busy() {
            // The review could not start; the field stays for another try.
            if let Some(inline) = &mut self.workspaces.rename.inline {
                inline.stage = Stage::Editing;
            }
        }
    }
    /// The slip's Rename: stage the reviewed change as one project Undo step.
    pub fn confirm_inline_rename(&mut self) {
        if let Some(inline) = &mut self.workspaces.rename.inline
            && inline.stage == Stage::Reviewed
        {
            inline.stage = Stage::Applying;
            self.apply_project_rename();
            if !self.project_jobs_busy()
                && let Some(inline) = &mut self.workspaces.rename.inline
            {
                inline.stage = Stage::Reviewed;
            }
        }
    }
    /// The slip's Review all changes: the rename page, with the review in place.
    pub fn review_inline_rename(&mut self) {
        if self
            .inline_rename()
            .is_some_and(|inline| inline.stage == Stage::Reviewed)
        {
            self.workspaces.rename.inline = None;
            self.open_project_workspace(Open::Rename);
        }
    }
    /// Esc or the slip's Cancel: the heading returns, the project untouched.
    pub fn cancel_inline_rename(&mut self) {
        let Some(inline) = &self.workspaces.rename.inline else {
            return;
        };
        if matches!(inline.stage, Stage::Reviewing | Stage::Applying) {
            return;
        }
        let reviewed = inline.stage == Stage::Reviewed;
        let beat = inline.beat.clone();
        self.end_inline_rename(beat);
        if reviewed {
            self.discard_rename_review();
        }
    }
    /// The field gives way to the name again, and focus returns to where it was typed.
    fn end_inline_rename(&mut self, beat: String) {
        let Some(inline) = self.workspaces.rename.inline.take() else {
            return;
        };
        if inline.place == Place::Heading {
            self.focus_pane(PaneMotion::To(Pane::Script));
        } else {
            self.workspaces.rename.returning = Some(Target {
                beat,
                place: inline.place,
            });
        }
    }
    /// A finished review or staging moves the inline rename on: a review shows its slip, a
    /// refused one opens the full page with the report, and a staged one ends it.
    pub(crate) fn settle_inline_rename(&mut self, succeeded: bool) {
        let Some(inline) = &mut self.workspaces.rename.inline else {
            return;
        };
        match (inline.stage, succeeded) {
            (Stage::Reviewing, true) => {
                inline.stage = Stage::Reviewed;
                self.status = "Rename reviewed".into();
                if inline.place == Place::Heading {
                    self.focus_pane(PaneMotion::To(Pane::Script));
                }
            }
            (Stage::Reviewing, false) => {
                self.workspaces.rename.inline = None;
                self.open_project_workspace(Open::Rename);
            }
            (Stage::Applying, true) => {
                let (beat, name) = (inline.beat.clone(), inline.name.trim().to_owned());
                self.end_inline_rename(name.clone());
                self.status = format!("Renamed {beat} to {name}. Save all writes the files.");
                self.workspaces.rename.undo_offer = Some(self.status.clone());
            }
            (Stage::Applying, false) => {
                self.workspaces.rename.inline = None;
                self.open_project_workspace(Open::Rename);
            }
            (Stage::Editing | Stage::Reviewed, _) => {}
        }
    }
    /// The status line offers Undo while it still reports an inline rename.
    pub fn offers_rename_undo(&self) -> bool {
        self.workspaces.rename.undo_offer.as_deref() == Some(self.status.as_str())
            && self.has_project_history(false)
    }
}

#[cfg(test)]
mod tests;
