use lsp_types::{
    DocumentChanges, OneOf, OptionalVersionedTextDocumentIdentifier, TextDocumentEdit, TextEdit,
    Uri, WorkspaceEdit,
};
use recite_compiler::{AuthoringEditPlan, AuthoringSnapshot, DocumentLayer, DocumentVersion};
use recite_core::DocumentKey;

use crate::position::source_range_to_lsp;

/// One exact source document available to an LSP projection.
///
/// The compiler owns the logical key, source bytes, and layer.  The LSP adds
/// only the URI and protocol version needed to form a workspace edit.
#[derive(Clone, Copy, Debug)]
pub(crate) struct EditDocument<'a> {
    pub(crate) key: &'a DocumentKey,
    pub(crate) uri: &'a Uri,
    pub(crate) text: &'a str,
    pub(crate) layer: DocumentLayer,
    pub(crate) version: Option<DocumentVersion>,
}

/// Projects a validated compiler edit plan into one atomic LSP workspace edit.
///
/// Every precondition document must have one URI, exact source text, and a
/// usable protocol version mapping before any edit is projected.  This keeps a
/// cross-document plan all-or-nothing when an editor has incomplete identity
/// or overlay state.
pub(crate) fn project_plan(
    plan: &AuthoringEditPlan,
    snapshot: &AuthoringSnapshot,
    documents: &[EditDocument<'_>],
) -> Option<WorkspaceEdit> {
    plan.validate(snapshot).ok()?;

    let mut changes = Vec::with_capacity(plan.preconditions().len());
    for precondition in plan.preconditions() {
        let document = unique_document(documents, precondition.document())?;
        precondition_matches(snapshot, precondition, document)?;
        let version = protocol_version(document)?;
        changes.push(PendingDocument {
            key: document.key.clone(),
            uri: document.uri.clone(),
            version,
            edits: Vec::new(),
        });
    }

    // A URI cannot safely represent two logical documents in one workspace
    // edit. Reject that mapping rather than emitting a partial or ambiguous
    // transaction, including for a precondition-only guarded document.
    for (index, change) in changes.iter().enumerate() {
        if changes[index + 1..]
            .iter()
            .any(|other| change.uri == other.uri && change.key != other.key)
        {
            return None;
        }
    }

    for edit in plan.edits() {
        let document = unique_document(documents, edit.document())?;
        let current = snapshot.document(document.key)?;
        if current.source_text() != document.text
            || current.layer() != document.layer
            || current.version() != document.version
        {
            return None;
        }
        let range = source_range_to_lsp(document.text, edit.range())?;
        let change = changes
            .iter_mut()
            .find(|change| change.key == *document.key)?;
        change.edits.push(TextEdit {
            range,
            new_text: edit.replacement().to_owned(),
        });
    }

    changes.sort_by(|left, right| left.uri.as_str().cmp(right.uri.as_str()));
    for change in &mut changes {
        // Plans already reject overlaps and are sorted in scalar source order.
        // Keep that ascending order in the protocol projection: LSP clients
        // apply a TextDocumentEdit as one non-overlapping transaction.
        change.edits.sort_by(|left, right| {
            left.range
                .start
                .line
                .cmp(&right.range.start.line)
                .then_with(|| left.range.start.character.cmp(&right.range.start.character))
                .then_with(|| left.range.end.line.cmp(&right.range.end.line))
                .then_with(|| left.range.end.character.cmp(&right.range.end.character))
                .then_with(|| left.new_text.cmp(&right.new_text))
        });
    }

    (!changes.is_empty()).then(|| WorkspaceEdit {
        changes: None,
        document_changes: Some(DocumentChanges::Edits(
            changes
                .into_iter()
                .map(|change| TextDocumentEdit {
                    // An empty edit list is intentional: LSP still carries
                    // the document version guard, preserving project-wide
                    // preconditions for plans whose edits target a sibling.
                    text_document: OptionalVersionedTextDocumentIdentifier {
                        uri: change.uri,
                        version: change.version,
                    },
                    edits: change.edits.into_iter().map(OneOf::Left).collect(),
                })
                .collect(),
        )),
        change_annotations: None,
    })
}

fn unique_document<'a>(
    documents: &'a [EditDocument<'a>],
    key: &DocumentKey,
) -> Option<&'a EditDocument<'a>> {
    let mut matches = documents.iter().filter(|document| *document.key == *key);
    let document = matches.next()?;
    matches.next().is_none().then_some(document)
}

fn protocol_version(document: &EditDocument<'_>) -> Option<Option<i32>> {
    match document.layer {
        DocumentLayer::Open => Some(Some(i32::try_from(document.version?.as_i64()).ok()?)),
        DocumentLayer::Saved => Some(None),
        _ => None,
    }
}

fn precondition_matches(
    snapshot: &AuthoringSnapshot,
    precondition: &recite_compiler::EditPrecondition,
    document: &EditDocument<'_>,
) -> Option<()> {
    let current = snapshot.document(document.key)?;
    if current.source_text() != document.text
        || current.layer() != document.layer
        || current.version() != document.version
        || current.version() != precondition.expected_version()
        || !precondition
            .source_fingerprint()
            .matches_source(document.text)
    {
        return None;
    }
    Some(())
}

struct PendingDocument {
    key: DocumentKey,
    uri: Uri,
    version: Option<i32>,
    edits: Vec<TextEdit>,
}
