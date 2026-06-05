use std::collections::BTreeSet;

use lsp_types::{
    DocumentChanges, GotoDefinitionResponse, Location, OneOf,
    OptionalVersionedTextDocumentIdentifier, Position, PrepareRenameResponse, Range,
    TextDocumentEdit, TextEdit, Uri, WorkspaceEdit,
};

use crate::position::span_to_range;
use crate::summary::{BlockReferenceSummary, FileSummary, SpannedName};

pub(crate) struct NavigationDocument<'a> {
    pub(crate) uri: &'a Uri,
    pub(crate) project_relative_path: Option<&'a str>,
    pub(crate) text: &'a str,
    pub(crate) summary: &'a FileSummary,
}

pub(crate) fn definition(
    uri: &Uri,
    position: Position,
    documents: &[NavigationDocument<'_>],
) -> Option<GotoDefinitionResponse> {
    let symbol = symbol_at(uri, position, documents)?;
    let definition = definition_for_symbol(&symbol, documents)?;
    Some(GotoDefinitionResponse::Scalar(location_for_definition(
        definition,
    )))
}

pub(crate) fn references(
    uri: &Uri,
    position: Position,
    include_declaration: bool,
    documents: &[NavigationDocument<'_>],
) -> Option<Vec<Location>> {
    let symbol = symbol_at(uri, position, documents)?;
    definition_for_symbol(&symbol, documents)?;

    let mut locations = Vec::new();
    if include_declaration {
        locations.extend(
            definitions_named(&symbol.name, documents)
                .into_iter()
                .map(location_for_definition),
        );
    }
    locations.extend(
        references_to_symbol(&symbol, documents)
            .into_iter()
            .map(location_for_reference),
    );

    Some(locations)
}

pub(crate) fn prepare_rename(
    uri: &Uri,
    position: Position,
    documents: &[NavigationDocument<'_>],
) -> Option<PrepareRenameResponse> {
    let symbol = symbol_at(uri, position, documents)?;
    definition_for_symbol(&symbol, documents)?;
    Some(PrepareRenameResponse::RangeWithPlaceholder {
        range: symbol.range,
        placeholder: symbol.name,
    })
}

pub(crate) fn rename(
    uri: &Uri,
    position: Position,
    new_name: &str,
    documents: &[NavigationDocument<'_>],
) -> Option<WorkspaceEdit> {
    if !is_block_symbol(new_name) {
        return None;
    }

    let symbol = symbol_at(uri, position, documents)?;
    definition_for_symbol(&symbol, documents)?;

    let mut changes = Vec::<(Uri, Vec<TextEdit>)>::new();
    for definition in definitions_named(&symbol.name, documents) {
        push_change(
            &mut changes,
            definition.document.uri.clone(),
            TextEdit {
                range: definition.range,
                new_text: new_name.to_owned(),
            },
        );
    }
    for reference in references_to_symbol(&symbol, documents) {
        push_change(
            &mut changes,
            reference.document.uri.clone(),
            TextEdit {
                range: reference.range,
                new_text: new_name.to_owned(),
            },
        );
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

struct Symbol {
    name: String,
    file: Option<String>,
    range: Range,
    kind: SymbolKind,
}

#[derive(Clone, Copy)]
enum SymbolKind {
    Definition,
    Reference,
}

struct Definition<'a> {
    document: &'a NavigationDocument<'a>,
    range: Range,
}

struct Reference<'a> {
    document: &'a NavigationDocument<'a>,
    range: Range,
}

fn symbol_at(
    uri: &Uri,
    position: Position,
    documents: &[NavigationDocument<'_>],
) -> Option<Symbol> {
    let document = documents.iter().find(|document| document.uri == uri)?;
    document
        .summary
        .blocks
        .iter()
        .find_map(|block| symbol_for_definition(block, document, position))
        .or_else(|| {
            document
                .summary
                .block_references
                .iter()
                .find_map(|reference| symbol_for_reference(reference, document, position))
        })
}

fn symbol_for_definition(
    block: &SpannedName,
    document: &NavigationDocument<'_>,
    position: Position,
) -> Option<Symbol> {
    let range = block_identifier_range(document.text, block);
    range_contains(range, position).then(|| Symbol {
        name: block.name.clone(),
        file: None,
        range,
        kind: SymbolKind::Definition,
    })
}

fn symbol_for_reference(
    reference: &BlockReferenceSummary,
    document: &NavigationDocument<'_>,
    position: Position,
) -> Option<Symbol> {
    let range = span_to_range(document.text, &reference.span);
    range_contains(range, position).then(|| Symbol {
        name: reference.block_id.clone(),
        file: reference.file.clone(),
        range,
        kind: SymbolKind::Reference,
    })
}

fn definition_for_symbol<'a>(
    symbol: &Symbol,
    documents: &'a [NavigationDocument<'a>],
) -> Option<Definition<'a>> {
    match symbol.kind {
        SymbolKind::Definition => unique_definition(&symbol.name, None, documents),
        SymbolKind::Reference => unique_definition(&symbol.name, symbol.file.as_deref(), documents),
    }
}

fn unique_definition<'a>(
    name: &str,
    file: Option<&str>,
    documents: &'a [NavigationDocument<'a>],
) -> Option<Definition<'a>> {
    let mut definitions = definitions_named_in_file(name, file, documents);
    let definition = definitions.next()?;
    definitions.next().is_none().then_some(definition)
}

fn definitions_named<'a>(
    name: &str,
    documents: &'a [NavigationDocument<'a>],
) -> Vec<Definition<'a>> {
    definitions_named_in_file(name, None, documents).collect()
}

fn definitions_named_in_file<'a>(
    name: &str,
    file: Option<&str>,
    documents: &'a [NavigationDocument<'a>],
) -> impl Iterator<Item = Definition<'a>> {
    documents
        .iter()
        .filter(move |document| file_matches(document, file))
        .flat_map(move |document| {
            document
                .summary
                .blocks
                .iter()
                .filter(move |block| block.name == name)
                .map(move |block| Definition {
                    document,
                    range: block_identifier_range(document.text, block),
                })
        })
}

fn references_to_symbol<'a>(
    symbol: &Symbol,
    documents: &'a [NavigationDocument<'a>],
) -> Vec<Reference<'a>> {
    let target_files = if let Some(file) = &symbol.file {
        BTreeSet::from([file.as_str()])
    } else {
        documents
            .iter()
            .filter(|document| {
                document
                    .summary
                    .blocks
                    .iter()
                    .any(|block| block.name == symbol.name)
            })
            .filter_map(|document| document.project_relative_path)
            .collect::<BTreeSet<_>>()
    };

    documents
        .iter()
        .flat_map(|document| {
            let target_files = &target_files;
            document
                .summary
                .block_references
                .iter()
                .filter(move |reference| {
                    reference.block_id == symbol.name
                        && match reference.file.as_deref() {
                            Some(file) => target_files.contains(file),
                            None => symbol.file.is_none(),
                        }
                })
                .map(move |reference| Reference {
                    document,
                    range: span_to_range(document.text, &reference.span),
                })
        })
        .collect()
}

fn file_matches(document: &NavigationDocument<'_>, file: Option<&str>) -> bool {
    file.is_none_or(|file| document.project_relative_path == Some(file))
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

fn location_for_definition(definition: Definition<'_>) -> Location {
    Location {
        uri: definition.document.uri.clone(),
        range: definition.range,
    }
}

fn location_for_reference(reference: Reference<'_>) -> Location {
    Location {
        uri: reference.document.uri.clone(),
        range: reference.range,
    }
}

fn block_identifier_range(text: &str, block: &SpannedName) -> Range {
    let line = text
        .lines()
        .nth(
            block
                .span
                .start
                .line()
                .saturating_sub(1)
                .try_into()
                .unwrap_or(usize::MAX),
        )
        .unwrap_or_default();
    let full_range = span_to_range(text, &block.span);
    let Some(start_byte) = line.find(block.name.as_str()) else {
        return full_range;
    };
    let end_byte = start_byte + block.name.len();
    Range {
        start: Position {
            line: full_range.start.line,
            character: utf16_units_for_byte_index(line, start_byte),
        },
        end: Position {
            line: full_range.start.line,
            character: utf16_units_for_byte_index(line, end_byte),
        },
    }
}

fn range_contains(range: Range, position: Position) -> bool {
    position.line == range.start.line
        && position.line == range.end.line
        && position.character >= range.start.character
        && position.character <= range.end.character
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

fn utf16_units_for_byte_index(line: &str, byte_index: usize) -> u32 {
    line.get(..byte_index)
        .unwrap_or(line)
        .chars()
        .map(char::len_utf16)
        .fold(0_u32, |total, width| {
            total.saturating_add(u32::try_from(width).unwrap_or(u32::MAX))
        })
}
