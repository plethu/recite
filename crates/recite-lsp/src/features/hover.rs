use lsp_types::{Hover, Position};
use recite_compiler::{AuthoringSnapshot, QueryResult};
use recite_core::{DocumentKey, ProjectSchema};
use recite_ui::UiCatalog;

use crate::position::lsp_position_to_source;

mod position;
mod schema;
mod typed;

pub(crate) use position::byte_index_for_utf16_character;

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

    if let Some(info) = typed_info {
        if let Some(value) = typed::typed_hover(key, snapshot, schema, &info, catalog) {
            return Some(value);
        }
        return None;
    }

    None
}
