//! The continuous scene galley: the scene's beats in order around the selected one, which alone
//! mounts editable fields. Read-only beats come from the same accepted projection as the selected
//! beat, so the galley never shows text the document does not hold.
use super::{Content, Reading, Row, flatten};
use crate::{Phase, Writer, navigation::Target};
use recite_writer_session::location::{Location, Screen};
use std::ops::Range;
use std::sync::Arc;

/// Beats set with their text at once. Longer scenes show a window around the selected beat; the
/// scene list and Ctrl+Up and Ctrl+Down reach the rest.
pub const GALLEY: usize = 12;

impl Reading {
    /// Every beat's rows, shaped once per accepted source.
    pub(super) fn shape_beats(&mut self) {
        self.beats = self
            .script
            .as_ref()
            .map(|blocks| blocks.iter().map(|block| flatten(&block.entries)).collect())
            .unwrap_or_default();
    }
    /// The beats the galley sets, by their index in the scene.
    pub fn galley(&self) -> Range<usize> {
        let total = self.beats.len();
        let selected = self.position.as_ref().map_or(0, |position| position.index);
        let start = selected
            .saturating_sub(GALLEY / 2)
            .min(total.saturating_sub(GALLEY));
        start..(start + GALLEY).min(total)
    }
    /// A beat's accepted rows by its index in the scene.
    pub fn beat_rows(&self, index: usize) -> Option<&Arc<[Row]>> {
        self.beats.get(index)
    }
}

impl Writer {
    /// Select the next or previous beat in the scene, as Ctrl+Down and Ctrl+Up do.
    pub fn step_beat(&mut self, forward: bool) {
        if self.phase != Phase::Writing || self.navigation.busy() || self.work.busy() {
            return;
        }
        let Ok(blocks) = self.reading.script() else {
            return;
        };
        let current = self
            .reading
            .position
            .as_ref()
            .map(|position| position.index);
        let next = match (current, forward) {
            (Some(index), true) => index + 1,
            (Some(index), false) => match index.checked_sub(1) {
                Some(index) => index,
                None => return,
            },
            (None, _) => 0,
        };
        if let Some(block) = blocks.get(next) {
            let id = block.id.clone();
            self.inspect(id);
        }
    }

    /// Mount a read-only beat's fields and put the caret in its first passage, as its "Edit beat"
    /// heading does. A beat without passages is selected as a whole.
    pub fn edit_beat(&mut self, index: usize) {
        let Some(id) = self
            .reading
            .script()
            .ok()
            .and_then(|blocks| blocks.get(index))
            .map(|block| block.id.clone())
        else {
            return;
        };
        let first = self.reading.beat_rows(index).and_then(|rows| {
            rows.iter().find_map(|row| match &row.content {
                Content::Passage { passage, .. } => Some(passage.id.clone()),
                _ => None,
            })
        });
        let Some(passage) = first else {
            self.inspect(id);
            return;
        };
        let location = Location {
            screen: Screen::Write,
            beat: Some(id),
            passage: Some(passage),
            source: false,
            view: Some(recite_config::WriterView::Script),
            page: 0,
            ..self.current_location()
        };
        self.navigate(Target::Selection(location));
    }

    /// The first problem in each beat of the open scene, by beat id, in source order. A problem
    /// belongs to the last beat that starts at or before its line.
    /// Bring the first beat with a problem into the galley and ask for focus on its wedge.
    pub fn reveal_first_problem(&mut self) {
        let Some((beat, _)) = self.beat_problems().into_iter().next() else {
            return;
        };
        let index = self
            .reading
            .script()
            .ok()
            .and_then(|blocks| blocks.iter().position(|block| block.id == beat));
        if let Some(index) = index.filter(|index| !self.reading.galley().contains(index)) {
            self.edit_beat(index);
        }
        self.attention.request_problem_focus();
    }
    pub fn beat_problems(&self) -> Vec<(String, String)> {
        let document = self.session.model().document();
        let diagnostics = document.diagnostic_snapshot();
        let blocks = document.block_summaries();
        let mut problems: Vec<(String, String)> = Vec::new();
        for diagnostic in (0..diagnostics.len()).filter_map(|index| diagnostics.get(index)) {
            let line = diagnostic.span.start.line();
            let Some(block) = blocks
                .iter()
                .filter(|block| {
                    block.span().file == diagnostic.span.file && block.span().start.line() <= line
                })
                .max_by_key(|block| block.span().start.line())
            else {
                continue;
            };
            let id = block.id().to_string();
            if problems.iter().all(|(beat, _)| beat != &id) {
                problems.push((id, diagnostic.message.clone()));
            }
        }
        problems
    }
}

#[cfg(test)]
mod tests;
