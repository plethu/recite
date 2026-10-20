// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0
// Local Recite experiment: basic bounded committed-text history, without timed grouping.
use std::collections::VecDeque;
use std::ops::Range;

const MAX_EDITS: usize = 100;
const MAX_BYTES: usize = 2 * 1024 * 1024;

pub(super) struct Snapshot {
    pub(super) text: String,
    pub(super) selection: [usize; 2],
}

struct Edit {
    start: usize,
    removed: String,
    inserted: String,
    before: [usize; 2],
    after: [usize; 2],
}

impl Edit {
    fn bytes(&self) -> usize {
        self.removed.len() + self.inserted.len()
    }
}

pub(super) struct Replay {
    pub(super) range: Range<usize>,
    pub(super) text: String,
    pub(super) selection: [usize; 2],
}

#[derive(Default)]
pub(super) struct History {
    undo: VecDeque<Edit>,
    redo: Vec<Edit>,
    bytes: usize,
}

impl History {
    pub(super) fn record(&mut self, before: Snapshot, after: &str, selection: [usize; 2]) {
        if before.text == after {
            return;
        }
        let mut start = before
            .text
            .bytes()
            .zip(after.bytes())
            .take_while(|(a, b)| a == b)
            .count();
        while !before.text.is_char_boundary(start) || !after.is_char_boundary(start) {
            start -= 1;
        }
        let mut suffix = before.text[start..]
            .bytes()
            .rev()
            .zip(after[start..].bytes().rev())
            .take_while(|(a, b)| a == b)
            .count();
        while !before.text.is_char_boundary(before.text.len() - suffix)
            || !after.is_char_boundary(after.len() - suffix)
        {
            suffix -= 1;
        }
        let edit = Edit {
            start,
            removed: before.text[start..before.text.len() - suffix].to_owned(),
            inserted: after[start..after.len() - suffix].to_owned(),
            before: before.selection,
            after: selection,
        };
        for retired in self.redo.drain(..) {
            self.bytes -= retired.bytes();
        }
        self.bytes += edit.bytes();
        self.undo.push_back(edit);
        while self.undo.len() > MAX_EDITS || self.bytes > MAX_BYTES {
            if let Some(retired) = self.undo.pop_front() {
                self.bytes -= retired.bytes();
            }
        }
    }

    pub(super) fn undo(&mut self) -> Option<Replay> {
        let edit = self.undo.pop_back()?;
        let replay = Replay {
            range: edit.start..edit.start + edit.inserted.len(),
            text: edit.removed.clone(),
            selection: edit.before,
        };
        self.redo.push(edit);
        Some(replay)
    }

    pub(super) fn redo(&mut self) -> Option<Replay> {
        let edit = self.redo.pop()?;
        let replay = Replay {
            range: edit.start..edit.start + edit.removed.len(),
            text: edit.inserted.clone(),
            selection: edit.after,
        };
        self.undo.push_back(edit);
        Some(replay)
    }
}
