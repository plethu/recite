//! Protocol text advances independently of cancellable analysis. Normalize each
//! accepted transaction before queueing so coalescing never drops a dependency.
use super::updates::Update;
use lsp_types::{Position, TextDocumentContentChangeEvent, Uri};
use std::collections::BTreeMap;

struct Document {
    version: i32,
    text: String,
}

#[derive(Default)]
pub(super) struct Documents(BTreeMap<Uri, Document>);

impl Documents {
    /// Rejected transactions change neither text nor version and must not
    /// invalidate outstanding requests. Analysis only receives full snapshots.
    pub(super) fn accept(&mut self, update: &mut Update) -> bool {
        match update {
            Update::Open(p) => {
                if self.0.contains_key(&p.text_document.uri) {
                    return false;
                }
                self.0.insert(
                    p.text_document.uri.clone(),
                    Document {
                        version: p.text_document.version,
                        text: p.text_document.text.clone(),
                    },
                );
            }
            Update::Change(p) => {
                let Some(document) = self.0.get_mut(&p.text_document.uri) else {
                    return false;
                };
                if p.text_document.version <= document.version {
                    return false;
                }
                let Some(text) = apply_changes(&document.text, &p.content_changes) else {
                    return false;
                };
                document.text.clone_from(&text);
                document.version = p.text_document.version;
                p.content_changes = vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text,
                }];
            }
            Update::Close(p) => return self.0.remove(&p.text_document.uri).is_some(),
            _ => {}
        }
        true
    }
}

fn apply_changes(original: &str, changes: &[TextDocumentContentChangeEvent]) -> Option<String> {
    let (first, rest) = changes.split_first()?;
    // Full replacements need not copy the previous document first.
    let mut text = if first.range.is_none() {
        if first.range_length.is_some() {
            return None;
        }
        first.text.clone()
    } else {
        let mut text = original.to_owned();
        apply_change(&mut text, first)?;
        text
    };
    for change in rest {
        apply_change(&mut text, change)?;
    }
    Some(text)
}

fn apply_change(text: &mut String, change: &TextDocumentContentChangeEvent) -> Option<()> {
    if let Some(range) = change.range {
        if range.start > range.end {
            return None;
        }
        let start = byte_offset(text, range.start)?;
        // Resolve the end from the already located start, avoiding a second
        // scan of the unchanged prefix for the usual small ranged edit.
        let relative_end = Position::new(
            range.end.line - range.start.line,
            if range.end.line == range.start.line {
                range.end.character - range.start.character
            } else {
                range.end.character
            },
        );
        let end = start + byte_offset(&text[start..], relative_end)?;
        if let Some(length) = change.range_length
            && usize::try_from(length).ok()? != text[start..end].encode_utf16().count()
        {
            return None;
        }
        text.replace_range(start..end, &change.text);
    } else {
        if change.range_length.is_some() {
            return None;
        }
        text.clone_from(&change.text);
    }
    Some(())
}

/// LSP lines recognize LF, CRLF and CR. Overlong characters clamp to line end;
/// a position inside a UTF-16 surrogate pair cannot identify a UTF-8 boundary.
fn byte_offset(text: &str, position: Position) -> Option<usize> {
    let mut start = 0;
    let bytes = text.as_bytes();
    for _ in 0..position.line {
        start += text[start..].find(['\r', '\n'])?;
        if bytes[start] == b'\r' && bytes.get(start + 1) == Some(&b'\n') {
            start += 1;
        }
        start += 1;
    }
    let mut units = 0;
    for (offset, ch) in text[start..].char_indices() {
        if units == position.character || ch == '\r' || ch == '\n' {
            return Some(start + offset);
        }
        units += u32::try_from(ch.len_utf16()).ok()?;
        if units > position.character {
            return None;
        }
    }
    Some(text.len())
}

#[cfg(test)]
mod tests;
