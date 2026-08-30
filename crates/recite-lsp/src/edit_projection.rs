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
#[derive(Clone, Debug)]
pub(crate) struct EditDocument {
    pub(crate) key: DocumentKey,
    pub(crate) uri: Uri,
    pub(crate) text: String,
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
    documents: &[EditDocument],
) -> Option<WorkspaceEdit> {
    plan.validate(snapshot).ok()?;

    let mut precondition_documents = Vec::with_capacity(plan.preconditions().len());
    for precondition in plan.preconditions() {
        let document = unique_document(documents, precondition.document())?;
        let version = protocol_version(document)?;
        precondition_documents.push((document, version));
    }

    // A URI cannot safely represent two logical documents in one workspace
    // edit.  Reject that mapping rather than emitting a partial or ambiguous
    // transaction.
    for (index, (document, _)) in precondition_documents.iter().enumerate() {
        if precondition_documents[index + 1..]
            .iter()
            .any(|(other, _)| document.uri == other.uri && document.key != other.key)
        {
            return None;
        }
    }

    let mut changes = Vec::<PendingDocument>::new();
    for edit in plan.edits() {
        let document = unique_document(documents, edit.document())?;
        let version = protocol_version(document)?;
        let range = source_range_to_lsp(&document.text, edit.range())?;
        let Some(change) = changes.iter_mut().find(|change| change.uri == document.uri) else {
            changes.push(PendingDocument {
                key: document.key.clone(),
                uri: document.uri.clone(),
                version,
                edits: vec![TextEdit {
                    range,
                    new_text: edit.replacement().to_owned(),
                }],
            });
            continue;
        };
        if change.key != document.key || change.version != version {
            return None;
        }
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
    documents: &'a [EditDocument],
    key: &DocumentKey,
) -> Option<&'a EditDocument> {
    let mut matches = documents.iter().filter(|document| document.key == *key);
    let document = matches.next()?;
    matches.next().is_none().then_some(document)
}

fn protocol_version(document: &EditDocument) -> Option<Option<i32>> {
    match document.layer {
        DocumentLayer::Open => Some(Some(i32::try_from(document.version?.as_i64()).ok()?)),
        DocumentLayer::Saved => Some(None),
        _ => None,
    }
}

struct PendingDocument {
    key: DocumentKey,
    uri: Uri,
    version: Option<i32>,
    edits: Vec<TextEdit>,
}
