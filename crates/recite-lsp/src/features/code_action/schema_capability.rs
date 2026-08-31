use lsp_types::{CodeAction, CodeActionKind, CodeActionOrCommand};
use recite_compiler::{
    SchemaAction, SchemaCapability, SchemaCapabilityUnavailableReason, SchemaSummary,
};
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
    let mut declared = Vec::new();
    add_capability(
        &mut declared,
        "schema",
        summary.capability(),
        summary.ownership().producer(),
    );
    for declaration in summary.types() {
        add_capability(
            &mut declared,
            &format!("type:{}", declaration.name()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
        );
    }
    for declaration in summary.registries() {
        add_capability(
            &mut declared,
            &format!("registry:{}", declaration.name()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
        );
    }
    for declaration in summary.speakers() {
        add_capability(
            &mut declared,
            &format!("speaker:{}", declaration.name()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
        );
    }
    for declaration in summary.conditions() {
        add_capability(
            &mut declared,
            &format!("condition:{}", declaration.name()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
        );
    }
    for declaration in summary.availability_reasons() {
        add_capability(
            &mut declared,
            &format!("reason:{}", declaration.id()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
        );
    }
    for declaration in summary.effects() {
        add_capability(
            &mut declared,
            &format!("effect:{}", declaration.name()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
        );
    }
    for declaration in summary.metadata_domains() {
        add_capability(
            &mut declared,
            &format!("metadata-domain:{}", declaration.name()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
        );
    }
    for declaration in summary.metadata() {
        add_capability(
            &mut declared,
            &format!("metadata:{}", declaration.name()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
        );
    }
    for declaration in summary.projection_queries() {
        add_capability(
            &mut declared,
            &format!("projection-query:{}", declaration.name()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
        );
    }
    for declaration in summary.presentation_projectors() {
        add_capability(
            &mut declared,
            &format!("projector:{}", declaration.name()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
        );
    }
    for declaration in summary.markup() {
        add_capability(
            &mut declared,
            &format!("markup:{}", declaration.name()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
        );
    }

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
                            UiArg::from(producer_name(action, declared.producer)),
                        ),
                        (
                            "declaration".to_owned(),
                            UiArg::from(declared.context.clone()),
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

struct DeclaredAction<'a> {
    context: String,
    action: &'a SchemaAction,
    producer: Option<&'a recite_core::ProducerIdentity>,
}

fn add_capability<'a>(
    declared: &mut Vec<DeclaredAction<'a>>,
    context: &str,
    capability: &'a SchemaCapability,
    producer: Option<&'a recite_core::ProducerIdentity>,
) {
    for action in capability.actions() {
        if declared
            .iter()
            .any(|candidate| candidate.context == context && candidate.action == action)
        {
            continue;
        }
        declared.push(DeclaredAction {
            context: context.to_owned(),
            action,
            producer,
        });
    }
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
) -> String {
    match action {
        SchemaAction::InvokeProducer { producer }
        | SchemaAction::RetryProducerFailure { producer } => {
            format!("{}/{}", producer.kind(), producer.id())
        }
        _ => declaration_producer.map_or_else(
            || "none".to_owned(),
            |producer| format!("{}/{}", producer.kind(), producer.id()),
        ),
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
