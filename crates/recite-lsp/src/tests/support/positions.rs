//! UTF-16 LSP positions located by text needles in test sources.

use lsp_types::Position;
use recite_core::StableId;

/// The position just after the first occurrence of `needle`.
pub(crate) fn position_after(source: &str, needle: &str) -> Position {
    position_for_byte_index(source, needle_start(source, needle) + needle.len())
}

/// The position one byte into the first occurrence of `needle`.
pub(crate) fn position_inside(source: &str, needle: &str) -> Position {
    position_for_byte_index(source, needle_start(source, needle) + 1)
}

/// The position just after `needle` within the line or choice header whose
/// stable ID is `id`, so tests address a header by identity rather than by
/// where its ID is spelled.
pub(crate) fn position_after_in_header(source: &str, id: &str, needle: &str) -> Position {
    let id = StableId::new(id).unwrap_or_else(|error| panic!("test ID `{id}`: {error}"));
    let field = format!(" {}", id.field());
    let (header_start, header) = source
        .split_inclusive('\n')
        .scan(0, |offset, line| {
            let start = *offset;
            *offset += line.len();
            Some((start, line))
        })
        .find(|(_, line)| line.trim_end().ends_with(&field))
        .unwrap_or_else(|| panic!("no header ends with `{field}`"));
    let index = header
        .find(needle)
        .unwrap_or_else(|| panic!("needle `{needle}` not in header `{header}`"));
    position_for_byte_index(source, header_start + index + needle.len())
}

fn needle_start(source: &str, needle: &str) -> usize {
    source
        .find(needle)
        .unwrap_or_else(|| panic!("needle not found: {needle}"))
}

fn position_for_byte_index(source: &str, byte_index: usize) -> Position {
    let mut line = 0_u32;
    let mut character = 0_u32;
    for value in source[..byte_index].chars() {
        if value == '\n' {
            line = line.saturating_add(1);
            character = 0;
        } else {
            character = character.saturating_add(value.len_utf16() as u32);
        }
    }
    Position::new(line, character)
}
