use recite_core::{DivertTarget, SourceId, SourcePosition, SourceText, Statement};
use recite_parser::{ReciteSyntaxKind, parse};
use std::ops::Range;

use crate::{DOCUMENT_NAME, EditError};

#[derive(Clone, Debug, PartialEq)]
pub enum PassageKind {
    Dialogue { speaker: Option<String> },
    Choice { destination: Option<String> },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Passage {
    pub id: String,
    pub label: String,
    pub section: String,
    pub text: String,
    pub kind: PassageKind,
    pub(crate) header_line: u32,
    pub(crate) text_range: Range<usize>,
    pub(crate) indentation: String,
    pub(crate) target_line: Option<u32>,
}

pub(crate) fn passages(source: &str) -> Result<Vec<Passage>, EditError> {
    let parsed = parse(DOCUMENT_NAME, source);
    let lowered = parsed.lower_source_file();
    if !lowered.diagnostics.is_empty() {
        return Err(EditError::SourceRequired("repair the syntax diagnostics"));
    }
    let mut result = Vec::new();
    for block in &lowered.source_file.blocks {
        let mut statements = Vec::new();
        for statement in &block.statements {
            statement.visit_depth_first(&mut |statement| statements.push(statement));
        }
        for statement in statements {
            let (id, text, kind, header_line, target_line) = match statement {
                Statement::Line(line) if line.plural_source_text.is_none() => (
                    &line.source_id,
                    &line.source_text,
                    PassageKind::Dialogue {
                        speaker: line
                            .speaker
                            .as_ref()
                            .or(block.default_speaker.as_ref())
                            .map(ToString::to_string),
                    },
                    line.span.start.line(),
                    None,
                ),
                Statement::Choice(choice) => (
                    &choice.source_id,
                    &choice.source_text,
                    PassageKind::Choice {
                        destination: choice.target.as_ref().map(|target| match &target.target {
                            DivertTarget::End => "END".to_owned(),
                            DivertTarget::Block(reference) => reference.file.as_ref().map_or_else(
                                || reference.block_id.to_string(),
                                |file| format!("{file}::{}", reference.block_id),
                            ),
                        }),
                    },
                    choice.span.start.line(),
                    choice
                        .target
                        .as_ref()
                        .map(|target| target.span.start.line()),
                ),
                _ => continue,
            };
            let SourceId::Frozen { label, anchor } = id else {
                return Err(EditError::MissingPassage);
            };
            if result
                .iter()
                .any(|passage: &Passage| passage.id == anchor.as_str())
            {
                return Err(EditError::MissingPassage);
            }
            let (text_range, indentation) = prose_range(source, text)?;
            result.push(Passage {
                id: anchor.to_string(),
                label: label.clone(),
                section: block.id.to_string(),
                text: text.text.clone(),
                kind,
                header_line,
                text_range,
                indentation,
                target_line,
            });
        }
    }
    Ok(result)
}

fn prose_range(source: &str, text: &SourceText) -> Result<(Range<usize>, String), EditError> {
    if text.text.is_empty() {
        return Err(EditError::SourceRequired("empty prose needs Source view"));
    }
    let first = usize::try_from(text.span.start.line() - 1).map_err(|_| EditError::Position)?;
    let count = text.text.split('\n').count();
    let parsed = parse(DOCUMENT_NAME, source);
    let nodes = parsed
        .syntax()
        .children()
        .skip(first)
        .take(count)
        .collect::<Vec<_>>();
    if nodes.len() != count
        || nodes
            .iter()
            .any(|node| node.kind() != ReciteSyntaxKind::Prose)
    {
        return Err(EditError::SourceRequired("the prose extent is ambiguous"));
    }
    let start = offset(source, text.span.start)?;
    let last = nodes.last().ok_or(EditError::Position)?;
    let last_token = last
        .children_with_tokens()
        .filter_map(|item| item.into_token())
        .find(|token| token.kind() == ReciteSyntaxKind::Text)
        .ok_or(EditError::Position)?;
    let end = usize::from(last_token.text_range().end());
    let header = source
        .split_inclusive('\n')
        .nth(first)
        .ok_or(EditError::Position)?;
    let indentation = header
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect();
    Ok((start..end, indentation))
}

pub(crate) fn offset(source: &str, position: SourcePosition) -> Result<usize, EditError> {
    let mut start = 0;
    for (index, line) in source.split_inclusive('\n').enumerate() {
        if index + 1 == usize::try_from(position.line()).map_err(|_| EditError::Position)? {
            let line = line.trim_end_matches(['\r', '\n']);
            let column = usize::try_from(position.column() - 1).map_err(|_| EditError::Position)?;
            let byte = if column == line.chars().count() {
                line.len()
            } else {
                line.char_indices()
                    .nth(column)
                    .map(|(byte, _)| byte)
                    .ok_or(EditError::Position)?
            };
            return Ok(start + byte);
        }
        start += line.len();
    }
    if position.column() == 1
        && source.ends_with('\n')
        && usize::try_from(position.line()).ok() == Some(source.lines().count() + 1)
    {
        return Ok(source.len());
    }
    Err(EditError::Position)
}
