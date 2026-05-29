use recite_core::{MarkupDefinition, SourcePosition, SourceSpan, SourceText};

use super::state::Validator;
use crate::diagnostics;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TagKind {
    Open,
    Close,
}

#[derive(Clone, Debug)]
struct MarkupTag<'a> {
    name: &'a str,
    kind: TagKind,
    span: SourceSpan,
}

#[derive(Clone, Debug)]
struct OpenTag<'a> {
    name: &'a str,
    definition: &'a MarkupDefinition,
    span: SourceSpan,
}

impl<'a> Validator<'a> {
    pub(super) fn validate_markup(&mut self, source_text: &'a SourceText) {
        let Some(schema) = self.schema else {
            return;
        };

        let mut open_tags = Vec::<OpenTag<'a>>::new();
        let mut offset = 0;
        while let Some(relative_open) = source_text.text[offset..].find('[') {
            let open = offset + relative_open;
            let Some(relative_close) = source_text.text[open..].find(']') else {
                let span = span_for_offset(source_text, open, 1);
                self.diagnostics.push(diagnostics::unbalanced_markup_tag(
                    "[",
                    span,
                    "missing closing bracket",
                    None,
                ));
                break;
            };
            let close = open + relative_close;
            offset = close + 1;

            let Some(tag) = parse_tag(source_text, open, close) else {
                continue;
            };
            let Some(definition) = schema.markup.get(tag.name) else {
                self.diagnostics
                    .push(diagnostics::unknown_markup_tag(tag.name, tag.span));
                continue;
            };

            match tag.kind {
                TagKind::Open => {
                    if let Some(parent) = open_tags
                        .iter()
                        .rev()
                        .find(|open_tag| !open_tag.definition.allows_nesting)
                    {
                        self.diagnostics.push(diagnostics::invalid_markup_nesting(
                            parent.name,
                            tag.name,
                            tag.span.clone(),
                            parent.span.clone(),
                        ));
                    }

                    if definition.requires_closing {
                        open_tags.push(OpenTag {
                            name: tag.name,
                            definition,
                            span: tag.span,
                        });
                    }
                }
                TagKind::Close => {
                    if !definition.requires_closing {
                        self.diagnostics.push(diagnostics::unbalanced_markup_tag(
                            tag.name,
                            tag.span,
                            "standalone tag does not use a closing tag",
                            None,
                        ));
                        continue;
                    }

                    let Some(open) = open_tags.last() else {
                        self.diagnostics.push(diagnostics::unbalanced_markup_tag(
                            tag.name,
                            tag.span,
                            "closing tag has no matching opening tag",
                            None,
                        ));
                        continue;
                    };

                    if open.name == tag.name {
                        open_tags.pop();
                    } else {
                        self.diagnostics.push(diagnostics::unbalanced_markup_tag(
                            tag.name,
                            tag.span,
                            format!("expected closing tag for `{}` first", open.name),
                            Some(open.span.clone()),
                        ));
                    }
                }
            }
        }

        for open in open_tags {
            self.diagnostics
                .push(diagnostics::missing_markup_closing_tag(
                    open.name, open.span,
                ));
        }
    }
}

fn parse_tag<'a>(source_text: &'a SourceText, open: usize, close: usize) -> Option<MarkupTag<'a>> {
    let raw_inner = &source_text.text[open + 1..close];
    let leading_ws = raw_inner.len() - raw_inner.trim_start().len();
    let inner = raw_inner.trim();
    let (kind, name_start_in_inner, name_source) =
        inner
            .strip_prefix('/')
            .map_or((TagKind::Open, 0, inner), |closing| {
                let trimmed = closing.trim_start();
                (TagKind::Close, 1 + closing.len() - trimmed.len(), trimmed)
            });
    let name_len = name_source
        .char_indices()
        .find_map(|(index, ch)| (!is_tag_name_char(ch)).then_some(index))
        .unwrap_or(name_source.len());
    let name = &name_source[..name_len];
    if name.is_empty() {
        return None;
    }

    let name_offset = open + 1 + leading_ws + name_start_in_inner;
    Some(MarkupTag {
        name,
        kind,
        span: span_for_offset(source_text, name_offset, name.len()),
    })
}

fn is_tag_name_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'
}

fn span_for_offset(source_text: &SourceText, offset: usize, len: usize) -> SourceSpan {
    let start = position_for_offset(source_text, offset);
    let end = (len > 0).then(|| position_for_offset(source_text, offset + len));
    SourceSpan::new(source_text.span.file.clone(), start, end)
}

// Invariant: offsets are within source text whose span starts at a valid source position.
#[allow(clippy::expect_used)]
fn position_for_offset(source_text: &SourceText, offset: usize) -> SourcePosition {
    let mut line = source_text.span.start.line();
    let mut column = source_text.span.start.column();
    for ch in source_text.text[..offset].chars() {
        if ch == '\n' {
            line += 1;
            column = source_text.span.start.column();
        } else {
            column += 1;
        }
    }
    SourcePosition::new(line, column).expect("source text offsets produce valid positions")
}
