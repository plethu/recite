//! Renaming the selected beat in place: its heading becomes a field, Enter reviews the project
//! rename, and a slip under the heading says what it changes before it is staged as one Undo
//! step. A rename the review refuses opens the full rename page with its report.
use super::*;
use crate::attention::{Pane, PaneMotion};
use crate::workspaces::{Open, Screen};

/// An inline rename of one beat, from the field to the staged change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inline {
    /// The beat's id when the rename began.
    pub beat: String,
    /// The new id as typed.
    pub name: String,
    pub stage: Stage,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// The heading is a field.
    Editing,
    /// The project rename is being reviewed.
    Reviewing,
    /// The review is ready: the consequence slip asks to rename or cancel.
    Reviewed,
    /// The reviewed rename is being staged.
    Applying,
}
/// What a reviewed rename changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Consequence {
    /// Changed source lines, the beat's own heading among them.
    pub references: usize,
    /// Changed files, the project manifest among them when its entry points move.
    pub files: usize,
}
impl Consequence {
    /// "Renames relay_desk to dock in 3 references across 2 files".
    pub fn sentence(self, beat: &str, name: &str) -> String {
        let plural = |count: usize, one: &str, many: &str| {
            format!("{count} {}", if count == 1 { one } else { many })
        };
        format!(
            "Renames {beat} to {name} in {} across {}",
            plural(self.references, "reference", "references"),
            plural(self.files, "file", "files"),
        )
    }
}

impl Writer {
    pub fn inline_rename(&self) -> Option<&Inline> {
        self.workspaces.rename.inline.as_ref()
    }
    /// Rename beat, from F2, the command or the heading's action: in place on the heading where
    /// the script shows the selected beat, otherwise on the rename page.
    pub fn rename_beat(&mut self) {
        let in_place = self.workspaces.screen == Screen::Write
            && self.map.workspace != crate::map::Workspace::Map
            && self.session.project().is_some()
            && self
                .session
                .model()
                .selected_block()
                .ok()
                .flatten()
                .is_some();
        if in_place {
            self.begin_inline_rename();
        } else {
            self.open_project_workspace(Open::Rename);
        }
    }
    /// Turn the selected beat's heading into a field holding its id; F2 and the heading's
    /// Rename beat action begin here.
    pub fn begin_inline_rename(&mut self) {
        if self.workspaces.screen != Screen::Write
            || self.session.project().is_none()
            || self.project_action_blocked()
            || self.inline_rename().is_some()
        {
            return;
        }
        let Some(beat) = self.session.model().selected_block().ok().flatten() else {
            return;
        };
        self.workspaces.rename.inline = Some(Inline {
            name: beat.clone(),
            beat,
            stage: Stage::Editing,
        });
        self.focus_pane(PaneMotion::To(Pane::Script));
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
        self.workspaces.rename.inline = None;
        if reviewed {
            self.discard_rename_review();
        }
        self.focus_pane(PaneMotion::To(Pane::Script));
    }
    /// What the reviewed rename changes, while its slip is showing.
    pub fn rename_consequence(&self) -> Option<Consequence> {
        let review = self.session.project()?.files().rename_review.as_ref()?;
        let changed = review
            .plan
            .changes
            .iter()
            .map(|change| changed_lines(&change.before, &change.after))
            .chain(std::iter::once(changed_lines(
                &review.manifest_before,
                &review.manifest_after,
            )))
            .filter(|lines| *lines > 0)
            .collect::<Vec<_>>();
        Some(Consequence {
            references: changed.iter().sum(),
            files: changed.len(),
        })
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
                self.focus_pane(PaneMotion::To(Pane::Script));
            }
            (Stage::Reviewing, false) => {
                self.workspaces.rename.inline = None;
                self.open_project_workspace(Open::Rename);
            }
            (Stage::Applying, true) => {
                let (beat, name) = (inline.beat.clone(), inline.name.trim().to_owned());
                self.workspaces.rename.inline = None;
                self.status = format!("Renamed {beat} to {name}. Save all writes the files.");
                self.workspaces.rename.undo_offer = Some(self.status.clone());
                self.focus_pane(PaneMotion::To(Pane::Script));
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

/// Lines that differ between two versions of a file a rename rewrites in place.
fn changed_lines(before: &str, after: &str) -> usize {
    let (mut old, mut new) = (before.lines(), after.lines());
    let mut count = 0;
    loop {
        match (old.next(), new.next()) {
            (None, None) => return count,
            (left, right) => count += usize::from(left != right),
        }
    }
}

#[cfg(test)]
mod tests;
