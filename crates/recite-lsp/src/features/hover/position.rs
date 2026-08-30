use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Range};

pub(super) fn hover_response(value: &str, range: Range) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::PlainText,
            value: value.to_owned(),
        }),
        range: Some(range),
    }
}

pub(crate) fn byte_index_for_utf16_character(line: &str, character: u32) -> Option<usize> {
    let mut utf16_units = 0_u32;
    for (byte_index, value) in line.char_indices() {
        if utf16_units == character {
            return Some(byte_index);
        }
        utf16_units = utf16_units.saturating_add(value.len_utf16() as u32);
        if utf16_units > character {
            return Some(byte_index);
        }
    }
    (utf16_units == character).then_some(line.len())
}
