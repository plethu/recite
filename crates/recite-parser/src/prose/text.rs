//! Checkable prose text with a map back to file byte ranges.

use std::ops::Range;

use super::{ProseRegion, ProseRegionKind};

/// The syntax a [`ProsePiece`] stands for in checkable text.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum ProsePieceKind {
    /// Prose copied verbatim from the source.
    Prose,
    /// An inline markup tag such as `[slow]`, removed from the checked text so
    /// the words around it keep their sentence context.
    Markup,
    /// An interpolation placeholder such as `{name}` (or a stray brace), kept
    /// verbatim but never checked as a word.
    Placeholder,
    /// An escape such as `\{`, replaced by the character it denotes.
    Escape,
    /// The newline and indentation between two body lines, checked as one
    /// newline, or as a blank line between paragraphs.
    Break,
}

/// One contiguous piece of checked text and the source bytes it came from.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProsePiece {
    kind: ProsePieceKind,
    text: Range<usize>,
    source: Range<usize>,
}

impl ProsePiece {
    #[must_use]
    pub const fn kind(&self) -> ProsePieceKind {
        self.kind
    }

    /// Byte range in [`ProseText::as_str`].
    #[must_use]
    pub fn text_range(&self) -> Range<usize> {
        self.text.clone()
    }

    /// Byte range in the source file.
    #[must_use]
    pub fn source_range(&self) -> Range<usize> {
        self.source.clone()
    }
}

/// Text to hand to a spelling or grammar checker, plus its source map.
///
/// Only [`ProsePieceKind::Prose`] pieces map one-to-one onto source bytes, so
/// only edits that fall inside one of them can be applied without touching
/// Recite syntax.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProseText {
    text: String,
    pieces: Vec<ProsePiece>,
}

impl ProseText {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub fn pieces(&self) -> &[ProsePiece] {
        &self.pieces
    }

    /// The exact source range of a checked-text range, when the range lies
    /// inside a single verbatim prose piece. Replacing that source range edits
    /// prose only.
    #[must_use]
    pub fn exact_source_range(&self, range: Range<usize>) -> Option<Range<usize>> {
        let piece = self.pieces.iter().find(|piece| {
            piece.kind == ProsePieceKind::Prose
                && piece.text.start <= range.start
                && range.end <= piece.text.end
                && range.start <= range.end
        })?;
        let start = piece.source.start + (range.start - piece.text.start);
        let end = piece.source.start + (range.end - piece.text.start);
        Some(start..end)
    }

    /// The smallest source range covering a checked-text range, for reporting
    /// a finding whose range crosses masked syntax or a line break.
    #[must_use]
    pub fn covering_source_range(&self, range: Range<usize>) -> Range<usize> {
        if let Some(exact) = self.exact_source_range(range.clone()) {
            return exact;
        }
        let fallback = self.pieces.first().map_or(0, |piece| piece.source.start);
        let start = self
            .pieces
            .iter()
            .find(|piece| piece.text.end > range.start)
            .map_or(fallback, |piece| match piece.kind {
                ProsePieceKind::Prose if range.start >= piece.text.start => {
                    piece.source.start + (range.start - piece.text.start)
                }
                _ => piece.source.start,
            });
        let end = self
            .pieces
            .iter()
            .rev()
            .find(|piece| piece.text.start < range.end)
            .map_or(start, |piece| match piece.kind {
                ProsePieceKind::Prose => {
                    piece.source.start + (range.end.min(piece.text.end) - piece.text.start)
                }
                _ => piece.source.end,
            });
        start..end.max(start)
    }

    /// Whether a checked-text range overlaps an interpolation placeholder.
    #[must_use]
    pub fn overlaps_placeholder(&self, range: Range<usize>) -> bool {
        self.pieces.iter().any(|piece| {
            piece.kind == ProsePieceKind::Placeholder
                && piece.text.start < range.end
                && range.start < piece.text.end
        })
    }

    fn push(&mut self, kind: ProsePieceKind, text: &str, source: Range<usize>) {
        let start = self.text.len();
        self.text.push_str(text);
        self.pieces.push(ProsePiece {
            kind,
            text: start..self.text.len(),
            source,
        });
    }
}

pub(super) fn build(region: &ProseRegion, source: &str) -> ProseText {
    let mut text = ProseText::default();
    let mut previous_end = None;
    let string_escapes = region.kind() == ProseRegionKind::Metadata;
    for line in region.lines() {
        let bytes = line.bytes();
        if let Some(previous_end) = previous_end {
            let separator = if line.paragraph_start() { "\n\n" } else { "\n" };
            text.push(ProsePieceKind::Break, separator, previous_end..bytes.start);
        }
        if let Some(slice) = source.get(bytes.clone()) {
            scan_line(&mut text, slice, bytes.start, string_escapes);
        }
        previous_end = Some(bytes.end);
    }
    text
}

/// Split one source line into prose and inline syntax. Placeholders and
/// escapes follow §5.10; markup follows the tag shape of §5.7, so bracketed
/// prose such as `[The door opens.]` stays checkable.
fn scan_line(text: &mut ProseText, line: &str, offset: usize, string_escapes: bool) {
    let mut prose_start = 0;
    let mut index = 0;
    let flush = |text: &mut ProseText, from: usize, to: usize| {
        if from < to {
            text.push(
                ProsePieceKind::Prose,
                &line[from..to],
                offset + from..offset + to,
            );
        }
    };
    while let Some(character) = line[index..].chars().next() {
        let width = character.len_utf8();
        let syntax = match character {
            '\\' => line[index + 1..].chars().next().and_then(|escaped| {
                (string_escapes || matches!(escaped, '{' | '}')).then(|| {
                    let mut buffer = [0; 4];
                    (
                        ProsePieceKind::Escape,
                        1 + escaped.len_utf8(),
                        escaped.encode_utf8(&mut buffer).to_owned(),
                    )
                })
            }),
            '{' => {
                let len = line[index..].find('}').map_or(1, |close| close + 1);
                Some((
                    ProsePieceKind::Placeholder,
                    len,
                    line[index..index + len].to_owned(),
                ))
            }
            '}' => Some((ProsePieceKind::Placeholder, 1, "}".to_owned())),
            '[' => markup_tag_len(&line[index..])
                .map(|len| (ProsePieceKind::Markup, len, String::new())),
            _ => None,
        };
        match syntax {
            Some((kind, len, replacement)) => {
                flush(text, prose_start, index);
                text.push(kind, &replacement, offset + index..offset + index + len);
                index += len;
                prose_start = index;
            }
            None => index += width,
        }
    }
    flush(text, prose_start, line.len());
}

/// The byte length of an inline markup tag at the start of `rest`: `[name]`,
/// `[/name]`, `[name=value]` or `[name key=value ...]` on one line.
fn markup_tag_len(rest: &str) -> Option<usize> {
    let close = rest.find(']')?;
    let inner = rest[1..close].trim();
    let mut tokens = inner.split_whitespace();
    let first = tokens.next()?;
    let first = first.strip_prefix('/').map_or(first, str::trim_start);
    let name_len = first
        .find(|character: char| !is_tag_name_char(character))
        .unwrap_or(first.len());
    let attributes_valid = tokens.all(|token| {
        token
            .split_once('=')
            .is_some_and(|(key, _)| !key.is_empty() && key.chars().all(is_tag_name_char))
    });
    let first_valid =
        name_len > 0 && (name_len == first.len() || first[name_len..].starts_with('='));
    (first_valid && attributes_valid).then_some(close + 1)
}

fn is_tag_name_char(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_' || character == '-'
}
