use recite_core::{Diagnostic, SourceRecovery};

mod diagnostics;
use rowan::{GreenNode, GreenNodeBuilder};
use std::sync::OnceLock;

use crate::layout::{ClassifiedLine, classify_line};
use crate::lower::{LoweredSourceFile, lower_source_file as lower_ast};
use crate::markers::StatementMarker;
use crate::source::{LogicalLine, LogicalLines};
use crate::syntax::{ReciteSyntaxKind, ReciteSyntaxNode};

/// Lossless parse output plus stable syntax diagnostics.
#[derive(Clone, Debug)]
pub struct Parse {
    path: String,
    source: String,
    green: OnceLock<GreenNode>,
    diagnostics: Vec<Diagnostic>,
    recovery: SourceRecovery,
    first_line: u32,
}

impl Parse {
    #[must_use]
    pub fn syntax(&self) -> ReciteSyntaxNode {
        let green = self.green.get_or_init(|| {
            let mut builder = GreenNodeBuilder::new();
            builder.start_node(ReciteSyntaxKind::Root.into());
            for line in LogicalLines::starting_at(&self.source, self.first_line) {
                parse_line(line, classify_line(line), &mut builder);
            }
            builder.finish_node();
            builder.finish()
        });
        ReciteSyntaxNode::new_root(green.clone())
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
        lower_ast(
            &self.path,
            &self.source,
            &self.diagnostics,
            self.recovery,
            self.first_line,
        )
    }
}

#[must_use]
pub fn parse(path: impl Into<String>, source: impl Into<String>) -> Parse {
    parse_region(path.into(), source.into(), 1)
}

/// Lower borrowed source with the same syntax diagnostics and recovery as [`parse`].
/// Use [`parse`] when a caller also needs a lossless syntax tree.
#[must_use]
pub fn lower_source_file(path: &str, source: &str) -> LoweredSourceFile {
    lower_region(path, source, 1)
}

pub(crate) fn lower_region(path: &str, source: &str, first_line: u32) -> LoweredSourceFile {
    let (diagnostics, recovery) = {
        let lines = LogicalLines::starting_at(source, first_line).collect::<Vec<_>>();
        diagnostics::scan(path, &lines)
    };
    lower_ast(path, source, &diagnostics, recovery, first_line)
}

pub(crate) fn parse_region(path: String, source: String, first_line: u32) -> Parse {
    let lines = LogicalLines::starting_at(&source, first_line).collect::<Vec<_>>();
    let (diagnostics, recovery) = diagnostics::scan(&path, &lines);

    Parse {
        path,
        source,
        green: OnceLock::new(),
        first_line,
        diagnostics,
        recovery,
    }
}

fn parse_line(
    line: LogicalLine<'_>,
    classification: ClassifiedLine,
    builder: &mut GreenNodeBuilder<'_>,
) {
    let indent = line.indentation();
    let trimmed = line.trimmed_content();
    let line_kind = classification.syntax_kind();

    builder.start_node(line_kind.into());
    push_token(builder, ReciteSyntaxKind::Whitespace, indent);

    match classification {
        ClassifiedLine::Statement(StatementMarker::Block) => {
            parse_prefixed_line(builder, trimmed, StatementMarker::Block);
        }
        ClassifiedLine::Statement(StatementMarker::Line) => {
            parse_prefixed_line(builder, trimmed, StatementMarker::Line);
        }
        ClassifiedLine::Statement(StatementMarker::Choice) => {
            parse_prefixed_line(builder, trimmed, StatementMarker::Choice);
        }
        ClassifiedLine::Statement(StatementMarker::Effect) => {
            parse_prefixed_line(builder, trimmed, StatementMarker::Effect);
        }
        ClassifiedLine::Statement(StatementMarker::Divert) => {
            parse_prefixed_line(builder, trimmed, StatementMarker::Divert);
        }
        ClassifiedLine::Statement(
            StatementMarker::If
            | StatementMarker::Else
            | StatementMarker::Match
            | StatementMarker::Case,
        ) => parse_directive_line(builder, trimmed),
        ClassifiedLine::Statement(StatementMarker::Comment) => parse_comment_line(builder, trimmed),
        ClassifiedLine::Blank | ClassifiedLine::Prose => {
            push_token(builder, ReciteSyntaxKind::Text, trimmed);
        }
        ClassifiedLine::Error => push_token(builder, ReciteSyntaxKind::Text, trimmed),
    }

    push_token(builder, ReciteSyntaxKind::Newline, line.newline);
    builder.finish_node();
}

fn parse_prefixed_line(builder: &mut GreenNodeBuilder<'_>, trimmed: &str, marker: StatementMarker) {
    push_marker(builder, marker);
    let rest = &trimmed[marker.text().len()..];
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

fn push_marker(builder: &mut GreenNodeBuilder<'_>, marker: StatementMarker) {
    push_token(builder, marker.marker_syntax_kind(), marker.text());
}

fn push_token(builder: &mut GreenNodeBuilder<'_>, kind: ReciteSyntaxKind, text: &str) {
    if !text.is_empty() {
        builder.token(kind.into(), text);
    }
}

#[cfg(test)]
mod tests;
