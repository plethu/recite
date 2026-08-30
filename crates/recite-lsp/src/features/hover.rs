use lsp_types::{Hover, Position};
use recite_compiler::{AuthoringSnapshot, QueryResult, SymbolKind};
use recite_core::{DocumentKey, ProjectSchema};
use recite_ui::{MsgId, UiCatalog};

use crate::position::lsp_position_to_source;

mod position;
mod schema;
mod typed;

pub(crate) use position::byte_index_for_utf16_character;
use position::{find_if_range, find_requires_range, hover_response};

pub(super) fn hover(
    text: &str,
    position: Position,
    key: &DocumentKey,
    snapshot: &AuthoringSnapshot,
    schema: Option<&ProjectSchema>,
    catalog: &UiCatalog,
) -> Option<Hover> {
    let source_position = lsp_position_to_source(text, position)?;
    let query = snapshot.hover(key, source_position);
    let typed_info = match query {
        QueryResult::Ready(info) | QueryResult::Partial { value: info, .. } => Some(info),
        QueryResult::NoMatch | QueryResult::Unavailable(_) => None,
        _ => None,
    };

    let line_index = usize::try_from(position.line).ok()?;
    let line = text.split('\n').nth(line_index)?;
    let line = line.strip_suffix('\r').unwrap_or(line);
    let byte_index = byte_index_for_utf16_character(line, position.character)?;

    if let Some(info) = typed_info {
        if let Some(value) = typed::typed_hover(key, snapshot, schema, &info, catalog) {
            return Some(value);
        }
        // These are LSP-owned explanatory ranges for syntax clauses.  The
        // semantic query still runs first, but the clause itself is not a
        // source symbol with a typed fact.
        if let Some(value) = syntax_clause_hover(line, line_index, byte_index, catalog) {
            return Some(value);
        }
        // A typed source location that failed projection must not fall
        // through to an unrelated same-named schema symbol in prose.
        if !matches!(info.location().kind(), SymbolKind::Schema) {
            return None;
        }
    } else if let Some(value) = syntax_clause_hover(line, line_index, byte_index, catalog) {
        return Some(value);
    }

    None
}

fn syntax_clause_hover(
    line: &str,
    line_index: usize,
    byte_index: usize,
    catalog: &UiCatalog,
) -> Option<Hover> {
    if let Some(range) = find_requires_range(line, line_index, byte_index) {
        return Some(hover_response(
            &catalog.text(MsgId::LspHoverRequires),
            range,
        ));
    }
    find_if_range(line, line_index, byte_index)
        .map(|range| hover_response(&catalog.text(MsgId::LspHoverIf), range))
}
