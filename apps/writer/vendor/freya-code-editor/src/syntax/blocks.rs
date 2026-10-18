//! Cached absolute rows and their relocation after edits.
use freya_core::prelude::Color;
use rustc_hash::FxHashMap;
use smallvec::SmallVec;
use std::ops::Range;
use tree_sitter::InputEdit;

pub enum TextNode {
    Range(Range<usize>),
    LineOfChars { len: usize, char: char },
}

pub type SyntaxLine = SmallVec<[(Color, TextNode); 4]>;

#[derive(Default)]
pub struct SyntaxBlocks {
    pub(super) blocks: FxHashMap<usize, SyntaxLine>,
}

impl SyntaxBlocks {
    pub fn push_line(&mut self, line: SyntaxLine) {
        self.blocks.insert(self.len(), line);
    }

    pub fn get_line_opt(&self, line: usize) -> Option<&[(Color, TextNode)]> {
        self.blocks.get(&line).map(|line| line.as_slice())
    }

    pub(crate) fn retain_rows(&mut self, rows: &Range<usize>) {
        self.blocks.retain(|row, _| rows.contains(row));
    }

    pub(crate) fn relocate(
        &mut self,
        edit: &InputEdit,
        old_chars: Range<usize>,
        new_end_char: usize,
    ) {
        let before = std::mem::take(&mut self.blocks);
        let first = edit.start_position.row.saturating_sub(1);
        let last = edit.old_end_position.row.saturating_add(1);
        for (row, mut line) in before {
            if row < first {
                self.blocks.insert(row, line);
            } else if row > last {
                let Some(row) = row
                    .checked_sub(edit.old_end_position.row)
                    .and_then(|n| n.checked_add(edit.new_end_position.row))
                else {
                    continue;
                };
                let valid = line.iter_mut().all(|(_, node)| {
                    let TextNode::Range(range) = node else {
                        return true;
                    };
                    let start = range
                        .start
                        .checked_sub(old_chars.end)
                        .and_then(|n| n.checked_add(new_end_char));
                    let end = range
                        .end
                        .checked_sub(old_chars.end)
                        .and_then(|n| n.checked_add(new_end_char));
                    let (Some(start), Some(end)) = (start, end) else {
                        return false;
                    };
                    *range = start..end;
                    true
                });
                if valid {
                    self.blocks.insert(row, line);
                }
            }
        }
    }

    pub fn get_line(&self, line: usize) -> &[(Color, TextNode)] {
        self.blocks.get(&line).unwrap()
    }

    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    pub fn clear(&mut self) {
        self.blocks.clear();
    }
}
