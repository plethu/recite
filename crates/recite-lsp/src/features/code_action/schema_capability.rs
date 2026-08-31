mod collect;

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
    let declared = collect::collect(summary);

    declared
        .iter()
        .filter_map(|declared| {
            let action = declared.action;
            if matches!(action, SchemaAction::EditStandaloneSource) && editable_source_open {
                return None;
            }
            Some(CodeActionOrCommand::CodeAction(CodeAction {
                title: catalog.format_args(
                    MsgId::LspCodeActionSchemaAction,
                    &UiArgs::from([
                        ("action".to_owned(), UiArg::from(action_name(action))),
                        (
                            "producer".to_owned(),
                            UiArg::from(producer_name(action, declared.producer, declared.origin)),
                        ),
                        (
                            "producer_state".to_owned(),
                            UiArg::from(producer_state(action, declared.producer, declared.origin)),
                        ),
                        (
                            "declaration_kind".to_owned(),
                            UiArg::from(declared.context.kind),
                        ),
                        (
                            "declaration_name".to_owned(),
                            UiArg::from(declared.context.name.clone().unwrap_or_default()),
                        ),
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
        SchemaAction::OpenSourceDeclaration => "open-source",
        SchemaAction::EditStandaloneSource => "edit-standalone",
        SchemaAction::InvokeProducer { .. } => "invoke",
        SchemaAction::RetryProducerFailure { .. } => "retry",
        SchemaAction::ReadOnlyGenerated => "read-only",
        SchemaAction::Unavailable { .. } => "unavailable",
        _ => "other",
    }
}

fn producer_name(
    action: &SchemaAction,
    declaration_producer: Option<&recite_core::ProducerIdentity>,
    declaration_origin: Option<&recite_core::ProducerOrigin>,
) -> String {
    match action {
        SchemaAction::InvokeProducer { producer }
        | SchemaAction::RetryProducerFailure { producer } => {
            format!("{}/{}", producer.kind(), producer.id())
        }
        _ => declaration_origin
            .map(|origin| format!("{}/{}", origin.kind, origin.id))
            .or_else(|| {
                declaration_producer
                    .map(|producer| format!("{}/{}", producer.kind(), producer.id()))
            })
            .unwrap_or_default(),
    }
}

fn producer_state(
    action: &SchemaAction,
    declaration_producer: Option<&recite_core::ProducerIdentity>,
    declaration_origin: Option<&recite_core::ProducerOrigin>,
) -> &'static str {
    if matches!(
        action,
        SchemaAction::InvokeProducer { .. } | SchemaAction::RetryProducerFailure { .. }
    ) || declaration_producer.is_some()
        || declaration_origin.is_some()
    {
        "present"
    } else {
        "absent"
    }
}

fn disabled_reason(action: &SchemaAction, editable_source_open: bool) -> &'static str {
    match action {
        SchemaAction::OpenSourceDeclaration => "source-location",
        SchemaAction::EditStandaloneSource if !editable_source_open => "standalone-source-closed",
        SchemaAction::EditStandaloneSource => "standalone-edit",
        SchemaAction::InvokeProducer { .. } => "producer-contract",
        SchemaAction::RetryProducerFailure { .. } => "producer-contract",
        SchemaAction::ReadOnlyGenerated => "generated-read-only",
        SchemaAction::Unavailable { reason } => unavailable_reason(*reason),
        _ => "other",
    }
}

fn unavailable_reason(reason: SchemaCapabilityUnavailableReason) -> &'static str {
    match reason {
        SchemaCapabilityUnavailableReason::UnknownSourceOwner => "unknown-source-owner",
        SchemaCapabilityUnavailableReason::ProducerCapabilityUnavailable => "producer-capability",
        _ => "other",
    }
}
