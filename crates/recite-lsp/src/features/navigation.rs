use lsp_types::{
    DocumentChanges, GotoDefinitionResponse, Location, OneOf,
    OptionalVersionedTextDocumentIdentifier, PrepareRenameResponse, Range, TextDocumentEdit,
    TextEdit, Uri, WorkspaceEdit,
};
use recite_compiler::{
    AuthoringSnapshot, NavigationResult, QueryResult, SymbolIdentity, SymbolLocation,
    SymbolQueryOptions, SymbolRole,
};
use recite_core::{DocumentKey, SourcePosition, SourceSpan};

use crate::position::span_to_range;

pub(crate) struct NavigationDocument<'a> {
    pub(crate) uri: &'a Uri,
    pub(crate) key: &'a DocumentKey,
    pub(crate) text: &'a str,
}

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
    let symbol = block_symbol(key, position, snapshot)?;
    unique_navigation(key, position, snapshot)?;
    if !matches!(
        snapshot.references(key, position, SymbolQueryOptions::default()),
        QueryResult::Ready(_)
    ) {
        return None;
    }
    let range = span_for_symbol(&symbol, snapshot)?;
    Some(PrepareRenameResponse::RangeWithPlaceholder {
        range,
        placeholder: block_name(&symbol)?.to_owned(),
    })
}

pub(crate) fn rename(
    key: &DocumentKey,
    position: SourcePosition,
    new_name: &str,
    snapshot: &AuthoringSnapshot,
    documents: &[NavigationDocument<'_>],
) -> Option<WorkspaceEdit> {
    // The compiler's typed authoring API currently stops at symbol references;
    // keep edit construction isolated at the LSP boundary until a typed edit
    // plan is available.
    if !is_block_symbol(new_name) {
        return None;
    }
    let block = block_symbol(key, position, snapshot)?;
    let destination = unique_navigation(key, position, snapshot)?;
    let old_name = block_name(&block)?;
    if destination_has_collision(snapshot, &destination, old_name, new_name) {
        return None;
    }
    let result = snapshot.references(key, position, SymbolQueryOptions::default());
    let locations = match result {
        QueryResult::Ready(locations) => locations,
        QueryResult::NoMatch | QueryResult::Unavailable(_) => return None,
        _ => return None,
    };
    if !locations
        .iter()
        .any(|location| location.role() == SymbolRole::Definition)
    {
        return None;
    }

    let mut changes = Vec::<(Uri, Vec<TextEdit>)>::new();
    for location in locations {
        let document = documents
            .iter()
            .find(|document| document.key == location.document())?;
        push_change(
            &mut changes,
            document.uri.clone(),
            TextEdit {
                range: span_to_range(document.text, location.span()),
                new_text: new_name.to_owned(),
            },
        );
    }
    if changes.is_empty() {
        return None;
    }
    changes.sort_by(|left, right| left.0.as_str().cmp(right.0.as_str()));
    for (_, edits) in &mut changes {
        edits.sort_by(|left, right| {
            left.range
                .start
                .line
                .cmp(&right.range.start.line)
                .then_with(|| left.range.start.character.cmp(&right.range.start.character))
        });
    }

    Some(WorkspaceEdit {
        changes: None,
        document_changes: Some(DocumentChanges::Edits(
            changes
                .into_iter()
                .map(|(uri, edits)| TextDocumentEdit {
                    text_document: OptionalVersionedTextDocumentIdentifier { uri, version: None },
                    edits: edits.into_iter().map(OneOf::Left).collect(),
                })
                .collect(),
        )),
        change_annotations: None,
    })
}

fn block_symbol(
    key: &DocumentKey,
    position: SourcePosition,
    snapshot: &AuthoringSnapshot,
) -> Option<SymbolLocation> {
    let result = snapshot.symbols(key, SymbolQueryOptions::default());
    let symbols = match result {
        QueryResult::Ready(symbols) | QueryResult::Partial { value: symbols, .. } => symbols,
        QueryResult::NoMatch | QueryResult::Unavailable(_) => return None,
        _ => return None,
    };
    symbols.into_iter().find(|symbol| {
        matches!(symbol.identity(), SymbolIdentity::Block(_))
            && span_contains(symbol.span(), position)
    })
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

fn destination_has_collision(
    snapshot: &AuthoringSnapshot,
    destination: &SymbolLocation,
    old_name: &str,
    new_name: &str,
) -> bool {
    if old_name == new_name {
        return false;
    }
    let QueryResult::Ready(symbols) =
        snapshot.symbols(destination.document(), SymbolQueryOptions::default())
    else {
        return true;
    };
    symbols.into_iter().any(|symbol| {
        symbol.role() == SymbolRole::Definition
            && matches!(symbol.identity(), SymbolIdentity::Block(name) if name.as_str() == new_name)
    })
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

fn span_for_symbol(symbol: &SymbolLocation, snapshot: &AuthoringSnapshot) -> Option<Range> {
    let document = snapshot.document(symbol.document())?;
    Some(span_to_range(document.source_text(), symbol.span()))
}

fn block_name(symbol: &SymbolLocation) -> Option<&str> {
    match symbol.identity() {
        SymbolIdentity::Block(name) => Some(name.as_str()),
        _ => None,
    }
}

fn span_contains(span: &SourceSpan, position: SourcePosition) -> bool {
    let Some(end) = span.end else {
        return false;
    };
    span.start <= position && position <= end
}

fn push_change(changes: &mut Vec<(Uri, Vec<TextEdit>)>, uri: Uri, edit: TextEdit) {
    if let Some((_, edits)) = changes
        .iter_mut()
        .find(|(existing_uri, _)| existing_uri == &uri)
    {
        edits.push(edit);
    } else {
        changes.push((uri, vec![edit]));
    }
}

fn is_block_symbol(value: &str) -> bool {
    !value.is_empty()
        && value != recite_core::END_DIVERT_TARGET
        && !value.contains("::")
        && value.chars().all(is_symbol_character)
}

fn is_symbol_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | ':')
}
