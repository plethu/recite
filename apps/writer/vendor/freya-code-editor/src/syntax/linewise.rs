//! Opt-in line-local projection, guarded by clean whole-file and changed-line parses.
use super::*;

// Large pastes are cheaper to parse once than to construct one Rope per line.
const MAX_CHANGED_ROWS: usize = 256;

pub(super) struct Linewise {
    parser: Box<SyntaxHighlighter>,
    source: Option<Rope>,
    pending: Option<InputEdit>,
}
impl Linewise {
    pub fn new(language: &EditorLanguage, theme: &EditorSyntaxTheme) -> Self {
        let mut language = language.clone();
        language.line_isolated = false;
        let mut parser = Box::new(SyntaxHighlighter::new());
        parser.set_language(Some(&language), theme);
        Self {
            parser,
            source: None,
            pending: None,
        }
    }
    pub fn invalidate(&mut self) {
        self.source = None;
        self.pending = None;
    }
    pub fn seed(&mut self, rope: &Rope, tree: &Tree) {
        self.invalidate();
        if clean(tree, rope) && supported_input(rope, tree) {
            self.source = Some(rope.clone());
        }
    }
    pub fn edit(&mut self, edit: &InputEdit) {
        // Coalesced edits have different coordinate systems: use the full parser.
        if self.pending.is_some() {
            self.invalidate();
        } else if self.source.is_some() {
            self.pending = Some(*edit);
        }
    }
    pub fn known_input(&self, rope: &Rope) -> bool {
        self.pending.is_some()
            || self
                .source
                .as_ref()
                .is_some_and(|old| old.is_instance(rope))
    }
    pub fn highlight(
        &mut self,
        rope: &Rope,
        blocks: &mut SyntaxBlocks,
        theme: &EditorSyntaxTheme,
        rows: Range<usize>,
        cancelled: &dyn Fn() -> bool,
    ) -> bool {
        if self.source.is_none() || !self.known_input(rope) {
            return false;
        }
        let mut local = SyntaxBlocks::default();
        if let Some(edit) = self.pending {
            let start = edit.start_position.row.saturating_sub(1);
            let end = edit
                .new_end_position
                .row
                .saturating_add(2)
                .min(rope.len_lines());
            if end.saturating_sub(start) > MAX_CHANGED_ROWS {
                return false;
            }
            for row in start..end {
                if !self.parse_line(rope, row, &mut local, theme, cancelled) {
                    return false;
                }
            }
        }
        let mut next = SyntaxBlocks::default();
        for row in rows {
            if !self.parse_line(rope, row, &mut local, theme, cancelled) {
                return false;
            }
            let offset = rope.line_to_char(row);
            if let Some(mut line) = local.blocks.remove(&0) {
                for (_, node) in &mut line {
                    if let TextNode::Range(range) = node {
                        range.start += offset;
                        range.end += offset;
                    }
                }
                next.blocks.insert(row, line);
            }
        }
        if cancelled() {
            return false;
        }
        *blocks = next;
        self.source = Some(rope.clone());
        self.pending = None;
        true
    }
    fn parse_line(
        &mut self,
        rope: &Rope,
        row: usize,
        blocks: &mut SyntaxBlocks,
        theme: &EditorSyntaxTheme,
        cancelled: &dyn Fn() -> bool,
    ) -> bool {
        if cancelled() {
            return false;
        }
        let line = rope.line(row);
        if !supported(line.chars()) {
            return false;
        }
        let input = Rope::from_str(&line.to_string());
        self.parser.invalidate_tree();
        self.parser
            .parse_cancellable(&input, blocks, theme, 0..1, cancelled)
            && self
                .parser
                .tree
                .as_ref()
                .is_some_and(|tree| clean(tree, &input))
    }
}
fn supported_input(rope: &Rope, tree: &Tree) -> bool {
    let root = tree.root_node();
    if root.end_byte() == rope.len_bytes() {
        // Tree-sitter counts LF; Ropey also counts bare CR and Unicode breaks.
        // CRLF contributes one to both. Extra Rope lines cannot cancel out.
        root.end_position().row.checked_add(1) == Some(rope.len_lines())
    } else {
        supported(rope.chars())
    }
}
fn supported(mut chars: impl Iterator<Item = char>) -> bool {
    while let Some(c) = chars.next() {
        match c {
            '\r' => {
                if chars.next() != Some('\n') {
                    return false;
                }
            }
            '\u{b}' | '\u{c}' | '\u{85}' | '\u{2028}' | '\u{2029}' => return false,
            _ => {}
        }
    }
    true
}
fn clean(tree: &Tree, rope: &Rope) -> bool {
    let root = tree.root_node();
    !root.has_error()
        && !root.is_missing()
        && rope
            .byte_slice(root.end_byte()..)
            .chars()
            .all(|c| c.is_ascii_whitespace())
}

#[cfg(test)]
#[path = "linewise/tests.rs"]
mod tests;
