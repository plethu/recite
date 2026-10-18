mod blocks;
mod config;
pub use blocks::{SyntaxBlocks, SyntaxLine, TextNode};
use config::LangConfig;

use std::{
    ops::{ControlFlow, Range},
    sync::atomic::{AtomicBool, Ordering},
};

use freya_core::prelude::Color;
use ropey::Rope;
use smallvec::SmallVec;
use tree_sitter::{InputEdit, Parser, Point, QueryCursor, StreamingIterator, Tree};

use crate::{editor_theme::EditorSyntaxTheme, languages::EditorLanguage};

pub struct SyntaxHighlighter {
    parser: Parser,
    tree: Option<Tree>,
    config: Option<LangConfig>,
    cursor: QueryCursor,
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
        }
    }

    /// Configures the language used for highlighting, or disables it with `None`.
    pub fn set_language(&mut self, language: Option<&EditorLanguage>, theme: &EditorSyntaxTheme) {
        self.parser.reset();
        self.tree = None;
        self.config = language.and_then(|language| language.lang_config(theme));
        if let Some(cfg) = &self.config {
            let _ = self.parser.set_language(&cfg.language);
        }
    }

    /// Discard the cached parse tree, forcing a full re-parse next time.
    pub fn invalidate_tree(&mut self) {
        self.parser.reset();
        self.tree = None;
    }

    /// Discard interrupted parser work before changing its input, then update
    /// the retained tree immediately. Multiple edits preserve their order.
    pub fn reset_parse(&mut self) {
        self.parser.reset();
    }

    pub fn edit(&mut self, edit: &InputEdit) {
        self.parser.reset();
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
        self.parse_cancellable(rope, syntax_blocks, theme, rows, None);
    }

    /// Worker shutdown interrupts parsing without blocking the UI on joining.
    pub(crate) fn parse_cancellable(
        &mut self,
        rope: &Rope,
        blocks: &mut SyntaxBlocks,
        theme: &EditorSyntaxTheme,
        rows: Range<usize>,
        closed: Option<&AtomicBool>,
    ) -> bool {
        let rows = rows.start.min(rope.len_lines())..rows.end.min(rope.len_lines());
        if self.config.is_none() {
            blocks.clear();
            build_plain_blocks(rope, blocks, theme, rows);
            return true;
        }
        let mut interrupted = false;
        let mut progress = |_: &tree_sitter::ParseState| {
            if closed.is_some_and(|closed| closed.load(Ordering::Relaxed)) {
                interrupted = true;
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        };
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

struct Span {
    start_byte: usize,
    end_byte: usize,
    color: Color,
}

fn build_syntax_blocks(
    tree: &Tree,
    cfg: &LangConfig,
    cursor: &mut QueryCursor,
    rope: &Rope,
    syntax_blocks: &mut SyntaxBlocks,
    theme: &EditorSyntaxTheme,
    rows: Range<usize>,
) {
    let root = tree.root_node();
    cursor.set_byte_range(rope.line_to_byte(rows.start)..rope.line_to_byte(rows.end));

    let mut spans: Vec<Span> = Vec::new();
    let mut captures = cursor.captures(&cfg.query, root, RopeTextProvider { rope });

    while let Some((match_result, capture_idx)) = {
        captures.advance();
        captures.get()
    } {
        let capture = &match_result.captures[*capture_idx];
        let node = capture.node;
        let color = cfg.capture_colors[capture.index as usize];
        spans.push(Span {
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
            color,
        });
    }

    spans.sort_by_key(|s| s.start_byte);
    build_lines_from_spans(rope, &spans, syntax_blocks, theme, rows);
}

fn build_lines_from_spans(
    rope: &Rope,
    spans: &[Span],
    syntax_blocks: &mut SyntaxBlocks,
    theme: &EditorSyntaxTheme,
    rows: Range<usize>,
) {
    let mut span_idx = 0;

    for line_idx in rows {
        let line_start_byte = rope.line_to_byte(line_idx);
        let line_slice = rope.line(line_idx);
        let line_byte_len = line_slice.len_bytes();
        let line_end_byte = line_start_byte + line_byte_len;

        let content_end_byte = {
            let chars = line_slice.len_chars();
            let mut end = line_end_byte;
            if chars > 0 && line_slice.char(chars - 1) == '\n' {
                end -= 1;
                if chars > 1 && line_slice.char(chars - 2) == '\r' {
                    end -= 1;
                }
            }
            end
        };

        while span_idx < spans.len() && spans[span_idx].end_byte <= line_start_byte {
            span_idx += 1;
        }

        let content_bytes = content_end_byte - line_start_byte;
        if content_bytes == 0 {
            syntax_blocks.blocks.insert(line_idx, SmallVec::new());
            continue;
        }

        let mut byte_colors: SmallVec<[Color; 256]> =
            smallvec::smallvec![theme.text; content_bytes];

        let mut si = span_idx;
        while si < spans.len() && spans[si].start_byte < content_end_byte {
            let span = &spans[si];
            si += 1;
            if span.end_byte <= line_start_byte {
                continue;
            }
            let s = span.start_byte.max(line_start_byte) - line_start_byte;
            let e = span.end_byte.min(content_end_byte) - line_start_byte;
            if s < e {
                for c in &mut byte_colors[s..e] {
                    *c = span.color;
                }
            }
        }

        let mut line_spans: SyntaxLine = SyntaxLine::new();
        let mut beginning_of_line = true;
        let mut run_start: usize = 0;

        while run_start < content_bytes {
            let run_color = byte_colors[run_start];
            let mut run_end = run_start + 1;
            while run_end < content_bytes && byte_colors[run_end] == run_color {
                run_end += 1;
            }

            let abs_start_byte = line_start_byte + run_start;
            let abs_end_byte = line_start_byte + run_end;
            let start_char = rope.byte_to_char(abs_start_byte);
            let end_char = rope.byte_to_char(abs_end_byte);

            if beginning_of_line {
                let slice = rope.slice(start_char..end_char);
                let is_whitespace = slice.chars().all(|c| c.is_whitespace() && c != '\n');
                if is_whitespace {
                    let len = end_char - start_char;
                    line_spans.push((
                        theme.whitespace,
                        TextNode::LineOfChars {
                            len,
                            char: '\u{00B7}',
                        },
                    ));
                    run_start = run_end;
                    continue;
                }
                beginning_of_line = false;
            }

            line_spans.push((run_color, TextNode::Range(start_char..end_char)));
            run_start = run_end;
        }

        syntax_blocks.blocks.insert(line_idx, line_spans);
    }
}

fn build_plain_blocks(
    rope: &Rope,
    syntax_blocks: &mut SyntaxBlocks,
    theme: &EditorSyntaxTheme,
    rows: Range<usize>,
) {
    for n in rows {
        let line = rope.line(n);
        let mut line_blocks = SmallVec::default();
        let start = rope.line_to_char(n);
        let end = line.len_chars();
        if end > 0 {
            line_blocks.push((theme.text, TextNode::Range(start..start + end)));
        }
        syntax_blocks.blocks.insert(n, line_blocks);
    }
}

pub struct RopeTextProvider<'a> {
    rope: &'a Rope,
}

impl<'a> tree_sitter::TextProvider<&'a [u8]> for RopeTextProvider<'a> {
    type I = RopeChunkIter<'a>;

    fn text(&mut self, node: tree_sitter::Node) -> Self::I {
        let start = node.start_byte();
        let end = node.end_byte();
        RopeChunkIter {
            rope: self.rope,
            byte_offset: start,
            end_byte: end,
        }
    }
}

pub struct RopeChunkIter<'a> {
    rope: &'a Rope,
    byte_offset: usize,
    end_byte: usize,
}

impl<'a> Iterator for RopeChunkIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.byte_offset >= self.end_byte {
            return None;
        }
        let (chunk, chunk_start, _, _) = self.rope.chunk_at_byte(self.byte_offset);
        let chunk_bytes = chunk.as_bytes();
        let offset_in_chunk = self.byte_offset - chunk_start;
        let available = &chunk_bytes[offset_in_chunk..];
        let remaining = self.end_byte - self.byte_offset;
        let slice = if available.len() > remaining {
            &available[..remaining]
        } else {
            available
        };
        self.byte_offset += slice.len();
        Some(slice)
    }
}
