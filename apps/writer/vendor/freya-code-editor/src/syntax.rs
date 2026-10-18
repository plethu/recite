mod blocks;
mod config;
mod linewise;
mod projection;
pub use blocks::{SyntaxBlocks, SyntaxLine, TextNode};
use config::LangConfig;
pub use projection::{RopeChunkIter, RopeTextProvider};
use projection::{build_plain_blocks, build_syntax_blocks};

use std::ops::{ControlFlow, Range};

use ropey::Rope;
use tree_sitter::{InputEdit, Parser, Point, QueryCursor, Tree};

use crate::{editor_theme::EditorSyntaxTheme, languages::EditorLanguage};

pub struct SyntaxHighlighter {
    parser: Parser,
    tree: Option<Tree>,
    config: Option<LangConfig>,
    cursor: QueryCursor,
    // An edited tree has current coordinates, but cannot be queried until parsed.
    tree_current: bool,
    input: Option<Rope>,
    pending_edits: bool,
    linewise: Option<linewise::Linewise>,
    #[cfg(test)]
    incremental_parses: usize,
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        Self {
            parser: Parser::new(),
            tree: None,
            config: None,
            cursor: QueryCursor::new(),
            tree_current: false,
            input: None,
            pending_edits: false,
            linewise: None,
            #[cfg(test)]
            incremental_parses: 0,
        }
    }

    /// Configures the language used for highlighting, or disables it with `None`.
    pub fn set_language(&mut self, language: Option<&EditorLanguage>, theme: &EditorSyntaxTheme) {
        self.parser.reset();
        self.tree = None;
        self.tree_current = false;
        self.input = None;
        self.pending_edits = false;
        self.linewise = language
            .filter(|language| language.line_isolated)
            .map(|language| linewise::Linewise::new(language, theme));
        self.config = language.and_then(|language| language.lang_config(theme));
        if let Some(cfg) = &self.config {
            let _ = self.parser.set_language(&cfg.language);
        }
    }

    pub(crate) fn has_tree(&self) -> bool {
        self.tree.is_some()
    }

    pub(crate) fn has_current_tree(&self) -> bool {
        self.tree_current
    }

    /// Discard the cached parse tree, forcing a full re-parse next time.
    pub fn invalidate_tree(&mut self) {
        self.parser.reset();
        self.tree = None;
        self.tree_current = false;
        self.input = None;
        self.pending_edits = false;
        if let Some(lines) = &mut self.linewise {
            lines.invalidate();
        }
    }

    /// Discard interrupted parser work before changing its input, then update
    /// the retained tree immediately. Multiple edits preserve their order.
    pub fn reset_parse(&mut self) {
        self.parser.reset();
        if let Some(lines) = &mut self.linewise {
            lines.invalidate();
        }
    }

    pub fn edit(&mut self, edit: &InputEdit) {
        self.parser.reset();
        self.tree_current = false;
        self.pending_edits = true;
        if let Some(lines) = &mut self.linewise {
            lines.edit(edit);
        }
        if let Some(tree) = &mut self.tree {
            tree.edit(edit);
        }
    }

    pub fn highlight_rows(
        &mut self,
        rope: &Rope,
        blocks: &mut SyntaxBlocks,
        theme: &EditorSyntaxTheme,
        rows: Range<usize>,
    ) {
        let start = rows.start.min(rope.len_lines());
        let rows = start..rows.end.max(start).min(rope.len_lines());
        if self.tree.is_some()
            && (!self.tree_current
                || self
                    .input
                    .as_ref()
                    .is_some_and(|old| !old.is_instance(rope)))
        {
            self.parse_cancellable(rope, blocks, theme, rows, || false);
            return;
        }
        blocks.clear();
        if let (Some(tree), Some(cfg)) = (&self.tree, &self.config) {
            build_syntax_blocks(tree, cfg, &mut self.cursor, rope, blocks, theme, rows);
        } else {
            build_plain_blocks(rope, blocks, theme, rows);
        }
    }

    /// Incrementally re-parse the rope and rebuild syntax blocks.
    pub fn parse(
        &mut self,
        rope: &Rope,
        syntax_blocks: &mut SyntaxBlocks,
        edit: Option<InputEdit>,
        theme: &EditorSyntaxTheme,
    ) {
        self.parse_visible(rope, syntax_blocks, edit, theme, 0..rope.len_lines());
    }

    /// Prototype: query the full tree but materialize only the requested rows.
    pub fn parse_visible(
        &mut self,
        rope: &Rope,
        syntax_blocks: &mut SyntaxBlocks,
        edit: Option<InputEdit>,
        theme: &EditorSyntaxTheme,
        rows: Range<usize>,
    ) {
        if let Some(edit) = edit {
            self.edit(&edit);
        }
        self.parse_cancellable(rope, syntax_blocks, theme, rows, || false);
    }

    /// Interrupt obsolete work or shutdown without blocking the editor thread.
    pub(crate) fn parse_cancellable(
        &mut self,
        rope: &Rope,
        blocks: &mut SyntaxBlocks,
        theme: &EditorSyntaxTheme,
        rows: Range<usize>,
        cancelled: impl Fn() -> bool,
    ) -> bool {
        let rows = rows.start.min(rope.len_lines())..rows.end.min(rope.len_lines());
        if self.config.is_none() {
            blocks.clear();
            build_plain_blocks(rope, blocks, theme, rows);
            return true;
        }
        if !self.pending_edits
            && self
                .input
                .as_ref()
                .is_some_and(|old| !old.is_instance(rope))
        {
            self.invalidate_tree();
        }
        self.input = Some(rope.clone());
        self.pending_edits = false;
        if let Some(lines) = &mut self.linewise {
            if lines.highlight(rope, blocks, theme, rows.clone(), &cancelled) {
                return true;
            }
            lines.invalidate();
        }
        if cancelled() {
            self.reset_parse();
            return false;
        }
        let mut interrupted = false;
        let mut progress = |_: &tree_sitter::ParseState| {
            if cancelled() {
                interrupted = true;
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        };
        #[cfg(test)]
        if self.tree.is_some() {
            self.incremental_parses += 1;
        }
        let len = rope.len_bytes();
        let new_tree = self.parser.parse_with_options(
            &mut |byte: usize, _: Point| {
                if byte >= len {
                    return &[] as &[u8];
                }
                let (chunk, start, _, _) = rope.chunk_at_byte(byte);
                &chunk.as_bytes()[byte - start..]
            },
            self.tree.as_ref(),
            Some(tree_sitter::ParseOptions::new().progress_callback(&mut progress)),
        );
        if let Some(tree) = new_tree {
            self.tree_current = true;
            if let Some(lines) = &mut self.linewise {
                lines.seed(rope, &tree);
            }
            self.tree = Some(tree);
            self.highlight_rows(rope, blocks, theme, rows);
            true
        } else if interrupted {
            false
        } else {
            blocks.clear();
            build_plain_blocks(rope, blocks, theme, rows);
            true
        }
    }
}

pub trait InputEditExt {
    fn new_edit(
        start_byte: usize,
        old_end_byte: usize,
        new_end_byte: usize,
        start_position: (usize, usize),
        old_end_position: (usize, usize),
        new_end_position: (usize, usize),
    ) -> InputEdit;
}

impl InputEditExt for InputEdit {
    fn new_edit(
        start_byte: usize,
        old_end_byte: usize,
        new_end_byte: usize,
        start_position: (usize, usize),
        old_end_position: (usize, usize),
        new_end_position: (usize, usize),
    ) -> InputEdit {
        InputEdit {
            start_byte,
            old_end_byte,
            new_end_byte,
            start_position: Point::new(start_position.0, start_position.1),
            old_end_position: Point::new(old_end_position.0, old_end_position.1),
            new_end_position: Point::new(new_end_position.0, new_end_position.1),
        }
    }
}

#[cfg(test)]
mod tests;
