use lsp_types::{GotoDefinitionResponse, Location, PrepareRenameResponse, WorkspaceEdit};
use recite_compiler::{
    AuthoringSnapshot, NavigationResult, QueryResult, SymbolIdentity, SymbolLocation,
    SymbolQueryOptions, SymbolRole,
};
use recite_core::{DocumentKey, SourcePosition};

use crate::edit_projection::{EditDocument, project_plan};
use crate::position::span_to_range;

pub(crate) type NavigationDocument<'a> = EditDocument<'a>;

pub(crate) fn definition(
    key: &DocumentKey,
    position: SourcePosition,
    snapshot: &AuthoringSnapshot,
    documents: &[NavigationDocument<'_>],
) -> Option<GotoDefinitionResponse> {
    let result = snapshot.navigate(key, position);
    let (QueryResult::Ready(NavigationResult::Unique(symbol))
    | QueryResult::Partial {
        value: NavigationResult::Unique(symbol),
        ..
    }) = result
    else {
        return None;
    };
    Some(GotoDefinitionResponse::Scalar(location_for_symbol(
        &symbol, documents,
    )?))
}

pub(crate) fn references(
    key: &DocumentKey,
    position: SourcePosition,
    include_declaration: bool,
    snapshot: &AuthoringSnapshot,
    documents: &[NavigationDocument<'_>],
) -> Option<Vec<Location>> {
    let QueryResult::Ready(NavigationResult::Unique(_)) = snapshot.navigate(key, position) else {
        return None;
    };
    let result = snapshot.references(key, position, SymbolQueryOptions::new(include_declaration));
    let locations = match result {
        QueryResult::Ready(locations) => locations,
        QueryResult::NoMatch | QueryResult::Unavailable(_) => return None,
        _ => return None,
    };
    // The former LSP projection returned declarations before references.  Keep
    // that protocol order while the compiler owns which occurrences exist.
    let mut declarations = Vec::new();
    let mut references = Vec::new();
    for symbol in locations {
        let location = location_for_symbol(&symbol, documents)?;
        if symbol.role() == SymbolRole::Definition {
            declarations.push(location);
        } else {
            references.push(location);
        }
    }
    declarations.extend(references);
    Some(declarations)
}

pub(crate) fn prepare_rename(
    key: &DocumentKey,
    position: SourcePosition,
    snapshot: &AuthoringSnapshot,
) -> Option<PrepareRenameResponse> {
    let symbol = symbol_at(key, position, snapshot)?;
    unique_navigation(key, position, snapshot)?;
    if !matches!(
        snapshot.references(key, position, SymbolQueryOptions::default()),
        QueryResult::Ready(_)
    ) {
        return None;
    }
    let document = snapshot.document(symbol.document())?;
    let range = span_to_range(document.source_text(), symbol.span());
    Some(PrepareRenameResponse::RangeWithPlaceholder {
        range,
        placeholder: block_name(&symbol)?.to_owned(),
    })
}

fn symbol_at(
    key: &DocumentKey,
    position: SourcePosition,
    snapshot: &AuthoringSnapshot,
) -> Option<SymbolLocation> {
    let QueryResult::Ready(symbols) = snapshot.symbols(key, SymbolQueryOptions::default()) else {
        return None;
    };
    let mut matches = symbols.into_iter().filter(|symbol| {
        matches!(symbol.identity(), SymbolIdentity::Block(_))
            && symbol
                .span()
                .end
                .is_some_and(|end| symbol.span().start <= position && position <= end)
    });
    let symbol = matches.next()?;
    matches.next().is_none().then_some(symbol)
}

pub(crate) fn rename(
    key: &DocumentKey,
    position: SourcePosition,
    new_name: &str,
    snapshot: &AuthoringSnapshot,
    documents: &[NavigationDocument<'_>],
) -> Option<WorkspaceEdit> {
    let plan = snapshot.plan_rename_block(key, position, new_name).ok()?;
    project_plan(&plan, snapshot, documents)
}

fn unique_navigation(
    key: &DocumentKey,
    position: SourcePosition,
    snapshot: &AuthoringSnapshot,
) -> Option<SymbolLocation> {
    let result = snapshot.navigate(key, position);
    match result {
        QueryResult::Ready(NavigationResult::Unique(symbol)) => Some(symbol),
        QueryResult::Ready(NavigationResult::Missing)
        | QueryResult::Ready(NavigationResult::Ambiguous(_))
        | QueryResult::Ready(NavigationResult::Unsupported)
        | QueryResult::Partial { .. }
        | QueryResult::Unavailable(_)
        | QueryResult::NoMatch
        | _ => None,
    }
}

fn location_for_symbol(
    symbol: &SymbolLocation,
    documents: &[NavigationDocument<'_>],
) -> Option<Location> {
    let document = documents
        .iter()
        .find(|document| document.key == symbol.document())?;
    Some(Location {
        uri: document.uri.clone(),
        range: span_to_range(document.text, symbol.span()),
    })
}

fn block_name(symbol: &SymbolLocation) -> Option<&str> {
    match symbol.identity() {
        SymbolIdentity::Block(name) => Some(name.as_str()),
        _ => None,
    }
}
