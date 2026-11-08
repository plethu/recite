use crate::workspace::LspWorkspace;
use lsp_server::{ErrorCode, Request, Response};
use lsp_types::request::{
    CodeActionRequest, Completion, GotoDefinition, HoverRequest, PrepareRenameRequest, References,
    Rename, Request as _,
};
use lsp_types::{
    CodeActionParams, CompletionParams, GotoDefinitionParams, HoverParams, ReferenceParams,
    RenameParams, TextDocumentPositionParams, Uri,
};
use recite_compiler::authoring::CancellationToken;

/// A query that completed without a result value.
#[derive(Debug, Clone, Eq, PartialEq)]
pub(super) struct QueryFailure {
    pub(super) code: i32,
    pub(super) message: String,
}

impl QueryFailure {
    fn internal(message: impl ToString) -> Self {
        Self {
            code: ErrorCode::InternalError as i32,
            message: message.to_string(),
        }
    }
}

pub(super) enum Query {
    Completion(CompletionParams),
    Hover(HoverParams),
    Definition(GotoDefinitionParams),
    References(ReferenceParams),
    PrepareRename(TextDocumentPositionParams),
    Rename(RenameParams),
    CodeAction(CodeActionParams),
}

impl Query {
    pub(super) fn parse(request: Request) -> Result<Self, Box<Response>> {
        let id = request.id.clone();
        let parsed = match request.method.as_str() {
            Completion::METHOD => request
                .extract(Completion::METHOD)
                .map(|(_, p)| Self::Completion(p)),
            HoverRequest::METHOD => request
                .extract(HoverRequest::METHOD)
                .map(|(_, p)| Self::Hover(p)),
            GotoDefinition::METHOD => request
                .extract(GotoDefinition::METHOD)
                .map(|(_, p)| Self::Definition(p)),
            References::METHOD => request
                .extract(References::METHOD)
                .map(|(_, p)| Self::References(p)),
            PrepareRenameRequest::METHOD => request
                .extract(PrepareRenameRequest::METHOD)
                .map(|(_, p)| Self::PrepareRename(p)),
            Rename::METHOD => request
                .extract(Rename::METHOD)
                .map(|(_, p)| Self::Rename(p)),
            CodeActionRequest::METHOD => request
                .extract(CodeActionRequest::METHOD)
                .map(|(_, p)| Self::CodeAction(p)),
            _ => {
                return Err(Box::new(Response::new_err(
                    id,
                    ErrorCode::MethodNotFound as i32,
                    format!("unsupported request method {}", request.method),
                )));
            }
        };
        parsed.map_err(|error| {
            Box::new(Response::new_err(
                id,
                ErrorCode::InvalidParams as i32,
                error.to_string(),
            ))
        })
    }

    pub(super) fn uri(&self) -> &Uri {
        match self {
            Self::Completion(p) => &p.text_document_position.text_document.uri,
            Self::Hover(p) => &p.text_document_position_params.text_document.uri,
            Self::Definition(p) => &p.text_document_position_params.text_document.uri,
            Self::References(p) => &p.text_document_position.text_document.uri,
            Self::PrepareRename(p) => &p.text_document.uri,
            Self::Rename(p) => &p.text_document_position.text_document.uri,
            Self::CodeAction(p) => &p.text_document.uri,
        }
    }

    pub(super) fn execute(
        &self,
        workspace: &LspWorkspace,
        control: &CancellationToken,
    ) -> Result<serde_json::Value, QueryFailure> {
        control.checkpoint().map_err(QueryFailure::internal)?;
        let result = match self {
            Self::Completion(p) => serde_json::to_value(workspace.completion_with_control(
                &p.text_document_position.text_document.uri,
                p.text_document_position.position,
                control,
            )),
            Self::Hover(p) => serde_json::to_value(workspace.hover_with_control(
                &p.text_document_position_params.text_document.uri,
                p.text_document_position_params.position,
                control,
            )),
            Self::Definition(p) => serde_json::to_value(workspace.definition_with_control(
                &p.text_document_position_params.text_document.uri,
                p.text_document_position_params.position,
                control,
            )),
            Self::References(p) => serde_json::to_value(workspace.references_with_control(
                &p.text_document_position.text_document.uri,
                p.text_document_position.position,
                p.context.include_declaration,
                control,
            )),
            Self::PrepareRename(p) => serde_json::to_value(workspace.prepare_rename_with_control(
                &p.text_document.uri,
                p.position,
                control,
            )),
            Self::Rename(p) => match workspace.rename_with_control(
                &p.text_document_position.text_document.uri,
                p.text_document_position.position,
                &p.new_name,
                control,
            ) {
                Ok(edit) => serde_json::to_value(edit),
                // A refused rename is the author's to fix, not a server fault.
                Err(refusal) => {
                    return Err(QueryFailure {
                        code: ErrorCode::RequestFailed as i32,
                        message: refusal.message,
                    });
                }
            },
            Self::CodeAction(p) => {
                serde_json::to_value(workspace.code_action_with_control(p, control))
            }
        };
        control.checkpoint().map_err(QueryFailure::internal)?;
        result.map_err(QueryFailure::internal)
    }
}
