use lsp_types::{Hover, Position};
use recite_compiler::authoring::{AuthoringQuery, QueryResult, SchemaSummary};
use recite_core::DocumentKey;
use recite_ui::UiCatalog;

use crate::position::indexed_lsp_position_to_source;

mod position;
mod schema;
mod schema_values;
mod typed;

pub(super) fn hover(
    position: Position,
    key: &DocumentKey,
    snapshot: &AuthoringQuery<'_>,
    schema: Option<&SchemaSummary>,
    catalog: &UiCatalog,
) -> Option<Hover> {
    let index = snapshot.document(key)?.source_index();
    let source_position = indexed_lsp_position_to_source(index, position)?;
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
