use lsp_types::{CodeAction, CodeActionKind, CodeActionOrCommand};
use recite_compiler::{SchemaAction, SchemaCapabilityUnavailableReason, SchemaSummary};
use recite_ui::{MsgId, UiArg, UiArgs, UiCatalog};

/// Project compiler capability descriptors into standard protocol actions.
///
/// The LSP owns neither producer processes nor source locations. Consequently
/// those actions remain visible but disabled until a future client contract
/// supplies the missing execution or navigation authority.
pub(crate) fn actions(
    summary: &SchemaSummary,
    editable_source_open: bool,
    catalog: &UiCatalog,
) -> Vec<CodeActionOrCommand> {
    summary
        .capability()
        .actions()
        .iter()
        .filter_map(|action| {
            if matches!(action, SchemaAction::EditStandaloneSource) && editable_source_open {
                return None;
            }
            Some(CodeActionOrCommand::CodeAction(CodeAction {
                title: catalog.format_args(
                    MsgId::LspCodeActionSchemaAction,
                    &UiArgs::from([
                        ("action".to_owned(), UiArg::from(action_name(action))),
                        ("producer".to_owned(), UiArg::from(producer_name(action))),
                    ]),
                ),
                kind: Some(CodeActionKind::REFACTOR),
                disabled: Some(lsp_types::CodeActionDisabled {
                    reason: catalog.format_args(
                        MsgId::LspCodeActionSchemaDisabled,
                        &UiArgs::from([(
                            "reason".to_owned(),
                            UiArg::from(disabled_reason(action, editable_source_open)),
                        )]),
                    ),
                }),
                ..CodeAction::default()
            }))
        })
        .collect()
}

fn action_name(action: &SchemaAction) -> &'static str {
    match action {
        SchemaAction::OpenSourceDeclaration => "open source declaration",
        SchemaAction::EditStandaloneSource => "edit standalone source",
        SchemaAction::InvokeProducer { .. } => "invoke producer",
        SchemaAction::RetryProducerFailure { .. } => "retry producer failure",
        SchemaAction::ReadOnlyGenerated => "read-only generated schema",
        SchemaAction::Unavailable { .. } => "unavailable schema action",
        _ => "future schema action",
    }
}

fn producer_name(action: &SchemaAction) -> String {
    match action {
        SchemaAction::InvokeProducer { producer }
        | SchemaAction::RetryProducerFailure { producer } => {
            format!("{}/{}", producer.kind(), producer.id())
        }
        _ => "none".to_owned(),
    }
}

fn disabled_reason(action: &SchemaAction, editable_source_open: bool) -> &'static str {
    match action {
        SchemaAction::OpenSourceDeclaration => "source location is not available",
        SchemaAction::EditStandaloneSource if !editable_source_open => {
            "standalone source is not open with a version"
        }
        SchemaAction::EditStandaloneSource => "standalone source edit is not available",
        SchemaAction::InvokeProducer { .. } => "producer execution contract is not available",
        SchemaAction::RetryProducerFailure { .. } => "producer execution contract is not available",
        SchemaAction::ReadOnlyGenerated => "generated schema is read-only",
        SchemaAction::Unavailable { reason } => unavailable_reason(*reason),
        _ => "schema action is not supported by this client",
    }
}

fn unavailable_reason(reason: SchemaCapabilityUnavailableReason) -> &'static str {
    match reason {
        SchemaCapabilityUnavailableReason::UnknownSourceOwner => "source owner is unknown",
        SchemaCapabilityUnavailableReason::ProducerCapabilityUnavailable => {
            "producer capability is unavailable"
        }
        _ => "schema capability is unavailable",
    }
}
