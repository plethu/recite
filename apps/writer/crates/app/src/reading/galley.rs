//! The continuous scene galley: the scene's beats in order around the selected one, which alone
//! mounts editable fields. Read-only beats come from the same accepted projection as the selected
//! beat, so the galley never shows text the document does not hold.
use super::{Reading, Row, flatten};
use crate::{Phase, Writer};
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

    /// The first problem in each beat of the open scene, by beat id, in source order. A problem
    /// belongs to the last beat that starts at or before its line.
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
