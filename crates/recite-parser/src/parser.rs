use recite_core::Diagnostic;
use rowan::{GreenNode, GreenNodeBuilder};

use crate::diagnostics::diagnostic;
use crate::lower::{LoweredSourceFile, lower_source_file};
use crate::source::{LogicalLine, LogicalLines, indent_len, span_for_line};
use crate::syntax::{ReciteSyntaxKind, ReciteSyntaxNode};

/// Lossless parse output plus stable syntax diagnostics.
#[derive(Clone, Debug)]
pub struct Parse {
    path: String,
    source: String,
    green: GreenNode,
    diagnostics: Vec<Diagnostic>,
}

impl Parse {
    #[must_use]
    pub fn syntax(&self) -> ReciteSyntaxNode {
        ReciteSyntaxNode::new_root(self.green.clone())
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Lowers recoverable syntax into the current `recite-core` source AST.
    ///
    /// Unsupported statement headers are retained in the syntax tree and
    /// reported as parser-boundary diagnostics until their lowering slices land.
    #[must_use]
    pub fn lower_source_file(&self) -> LoweredSourceFile {
        lower_source_file(&self.path, &self.source, &self.diagnostics)
    }
}

#[must_use]
pub fn parse(path: impl Into<String>, source: impl Into<String>) -> Parse {
    let path = path.into();
    let source = source.into();
    let mut builder = GreenNodeBuilder::new();
    let mut diagnostics = Vec::new();

    builder.start_node(ReciteSyntaxKind::Root.into());
    for logical_line in LogicalLines::new(&source) {
        parse_line(&path, logical_line, &mut builder, &mut diagnostics);
    }
    builder.finish_node();

    Parse {
        path,
        source,
        green: builder.finish(),
        diagnostics,
    }
}

fn parse_line(
    path: &str,
    line: LogicalLine<'_>,
    builder: &mut GreenNodeBuilder<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let content = line.content_without_newline();
    let indent_len = indent_len(content);
    let indent = &content[..indent_len];
    let trimmed = &content[indent_len..];
    let line_kind = classify_line(trimmed, indent_len);

    builder.start_node(line_kind.into());
    push_token(builder, ReciteSyntaxKind::Whitespace, indent);

    match line_kind {
        ReciteSyntaxKind::Block => parse_prefixed_line(builder, trimmed, "::"),
        ReciteSyntaxKind::Line => parse_prefixed_line(builder, trimmed, ">"),
        ReciteSyntaxKind::Choice => parse_prefixed_line(builder, trimmed, "?"),
        ReciteSyntaxKind::Effect => parse_prefixed_line(builder, trimmed, "!"),
        ReciteSyntaxKind::Divert => parse_prefixed_line(builder, trimmed, "->"),
        ReciteSyntaxKind::If
        | ReciteSyntaxKind::Else
        | ReciteSyntaxKind::Match
        | ReciteSyntaxKind::Case => parse_directive_line(builder, trimmed),
        ReciteSyntaxKind::Comment => parse_comment_line(builder, trimmed),
        ReciteSyntaxKind::Prose => push_token(builder, ReciteSyntaxKind::Text, trimmed),
        ReciteSyntaxKind::Error => {
            push_token(builder, ReciteSyntaxKind::Text, trimmed);
            if !trimmed.is_empty() {
                diagnostics.push(diagnostic(
                    "RECITE_PARSE001",
                    "expected a Recite statement header or indented prose",
                    span_for_line(path, line.number, indent_len + 1),
                ));
            }
        }
        _ => unreachable!("line classification must produce a node kind"),
    }

    push_token(builder, ReciteSyntaxKind::Newline, line.newline);
    builder.finish_node();
}

fn classify_line(trimmed: &str, indent_len: usize) -> ReciteSyntaxKind {
    if trimmed.is_empty() {
        return ReciteSyntaxKind::Prose;
    }

    if trimmed.starts_with("::") {
        return ReciteSyntaxKind::Block;
    }

    if trimmed.starts_with("->") {
        return ReciteSyntaxKind::Divert;
    }

    match trimmed.as_bytes()[0] {
        b'>' => ReciteSyntaxKind::Line,
        b'?' => ReciteSyntaxKind::Choice,
        b'!' => ReciteSyntaxKind::Effect,
        b'#' => ReciteSyntaxKind::Comment,
        b':' if trimmed.starts_with(":if") => ReciteSyntaxKind::If,
        b':' if trimmed.starts_with(":else") => ReciteSyntaxKind::Else,
        b':' if trimmed.starts_with(":match") => ReciteSyntaxKind::Match,
        b':' if trimmed.starts_with(":case") => ReciteSyntaxKind::Case,
        _ if indent_len > 0 => ReciteSyntaxKind::Prose,
        _ => ReciteSyntaxKind::Error,
    }
}

fn parse_prefixed_line(builder: &mut GreenNodeBuilder<'_>, trimmed: &str, marker: &str) {
    push_marker(builder, marker);
    let rest = &trimmed[marker.len()..];
    parse_header_rest(builder, rest);
}

fn parse_directive_line(builder: &mut GreenNodeBuilder<'_>, trimmed: &str) {
    let marker_len = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
    let (marker, rest) = trimmed.split_at(marker_len);

    push_token(builder, ReciteSyntaxKind::DirectiveMarker, marker);
    parse_header_rest(builder, rest);
}

fn parse_comment_line(builder: &mut GreenNodeBuilder<'_>, trimmed: &str) {
    push_token(builder, ReciteSyntaxKind::CommentText, trimmed);
}

fn parse_header_rest(builder: &mut GreenNodeBuilder<'_>, rest: &str) {
    let whitespace_len = rest.len() - rest.trim_start_matches([' ', '\t']).len();
    let (whitespace, text) = rest.split_at(whitespace_len);
    push_token(builder, ReciteSyntaxKind::Whitespace, whitespace);

    if let Some((ident, remaining)) = split_first_word(text) {
        push_token(builder, ReciteSyntaxKind::Ident, ident);
        push_token(builder, ReciteSyntaxKind::Text, remaining);
    } else {
        push_token(builder, ReciteSyntaxKind::Text, text);
    }
}

fn split_first_word(text: &str) -> Option<(&str, &str)> {
    let trimmed_len = text.len() - text.trim_start_matches([' ', '\t']).len();
    if trimmed_len > 0 {
        return None;
    }

    let first_whitespace = text.find(char::is_whitespace).unwrap_or(text.len());
    if first_whitespace == 0 {
        return None;
    }

    Some(text.split_at(first_whitespace))
}

fn push_marker(builder: &mut GreenNodeBuilder<'_>, marker: &str) {
    let kind = match marker {
        "::" => ReciteSyntaxKind::BlockMarker,
        ">" => ReciteSyntaxKind::LineMarker,
        "?" => ReciteSyntaxKind::ChoiceMarker,
        "!" => ReciteSyntaxKind::EffectMarker,
        "->" => ReciteSyntaxKind::DivertMarker,
        _ => ReciteSyntaxKind::Text,
    };

    push_token(builder, kind, marker);
}

fn push_token(builder: &mut GreenNodeBuilder<'_>, kind: ReciteSyntaxKind, text: &str) {
    if !text.is_empty() {
        builder.token(kind.into(), text);
    }
}
