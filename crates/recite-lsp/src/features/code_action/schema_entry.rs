use lsp_types::{CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, Range};
use recite_core::{
    ConditionDefinition, ConditionReturnType, EffectDefinition, EffectMode, SchemaSourceEdit,
};
use recite_ui::{MsgId, UiCatalog};

use super::{
    CodeActionDocument, SchemaCodeActionDocument, ranges_intersect, schema_workspace_edit,
};
use crate::summary::{FunctionReferenceKind, FunctionReferenceSummary};

pub(super) fn actions(
    params: &CodeActionParams,
    document: &CodeActionDocument<'_>,
    documents: &[CodeActionDocument<'_>],
    schema: &SchemaCodeActionDocument,
    catalog: &UiCatalog,
) -> Vec<CodeActionOrCommand> {
    if !schema
        .summary
        .capability()
        .actions()
        .iter()
        .any(|action| matches!(action, recite_compiler::SchemaAction::EditStandaloneSource))
    {
        return Vec::new();
    }
    if documents.iter().any(|document| {
        !document.summary.completeness.condition_functions
            || !document.summary.completeness.effect_functions
    }) {
        return Vec::new();
    }

    let mut actions = Vec::new();
    actions.extend(
        document
            .summary
            .condition_functions
            .iter()
            .filter(|function| function_intersects(document.source.text, function, params.range))
            .filter_map(|function| condition_action(params, function, documents, schema, catalog)),
    );
    actions.extend(
        document
            .summary
            .effect_functions
            .iter()
            .filter(|function| function_intersects(document.source.text, function, params.range))
            .filter_map(|function| effect_action(params, function, documents, schema, catalog)),
    );
    actions
}

fn function_intersects(text: &str, function: &FunctionReferenceSummary, range: Range) -> bool {
    ranges_intersect(range, crate::position::span_to_range(text, &function.span))
}

fn condition_action(
    params: &CodeActionParams,
    function: &FunctionReferenceSummary,
    documents: &[CodeActionDocument<'_>],
    schema: &SchemaCodeActionDocument,
    catalog: &UiCatalog,
) -> Option<CodeActionOrCommand> {
    if function.argument_count != 0 || function.kind != FunctionReferenceKind::BoolCondition {
        return None;
    }
    if same_name_conditions(documents, &function.name).any(|candidate| {
        candidate.argument_count != 0 || candidate.kind != FunctionReferenceKind::BoolCondition
    }) {
        return None;
    }
    if schema
        .summary
        .conditions()
        .iter()
        .any(|condition| condition.name() == function.name)
    {
        return None;
    }
    let edit = schema
        .source
        .plan_edit(SchemaSourceEdit::AddCondition {
            name: function.name.clone(),
            definition: ConditionDefinition {
                params: Vec::new(),
                returns: ConditionReturnType::Bool,
                availability_reason: None,
            },
        })
        .ok()?;
    schema_code_action(
        params,
        schema,
        documents,
        &edit,
        catalog.format_pairs(
            MsgId::LspCodeActionAddCondition,
            [("name", function.name.as_str())],
        ),
    )
}

fn effect_action(
    params: &CodeActionParams,
    function: &FunctionReferenceSummary,
    documents: &[CodeActionDocument<'_>],
    schema: &SchemaCodeActionDocument,
    catalog: &UiCatalog,
) -> Option<CodeActionOrCommand> {
    if function.argument_count != 0 {
        return None;
    }
    let modes = same_name_effect_modes(documents, &function.name)?;
    if schema
        .summary
        .effects()
        .iter()
        .any(|effect| effect.name() == function.name)
    {
        return None;
    }
    let modes = modes
        .iter()
        .filter_map(|mode| match *mode {
            "deferred" => Some(EffectMode::Deferred),
            "immediate" => Some(EffectMode::Immediate),
            "blocking" => Some(EffectMode::Blocking),
            _ => None,
        })
        .collect();
    let edit = schema
        .source
        .plan_edit(SchemaSourceEdit::AddEffect {
            name: function.name.clone(),
            definition: EffectDefinition {
                modes,
                params: Vec::new(),
            },
        })
        .ok()?;
    schema_code_action(
        params,
        schema,
        documents,
        &edit,
        catalog.format_pairs(
            MsgId::LspCodeActionAddEffect,
            [("name", function.name.as_str())],
        ),
    )
}

fn same_name_conditions<'a>(
    documents: &'a [CodeActionDocument<'_>],
    name: &'a str,
) -> impl Iterator<Item = &'a FunctionReferenceSummary> + 'a {
    documents
        .iter()
        .flat_map(|document| document.summary.condition_functions.iter())
        .filter(move |function| function.name == name)
}

fn same_name_effect_modes(
    documents: &[CodeActionDocument<'_>],
    name: &str,
) -> Option<Vec<&'static str>> {
    let mut deferred = false;
    let mut immediate = false;
    let mut blocking = false;
    for function in documents
        .iter()
        .flat_map(|document| document.summary.effect_functions.iter())
        .filter(|function| function.name == name)
    {
        if function.argument_count != 0 {
            return None;
        }
        match function.kind {
            FunctionReferenceKind::DeferredEffect => deferred = true,
            FunctionReferenceKind::ImmediateEffect => immediate = true,
            FunctionReferenceKind::BlockingEffect => blocking = true,
            FunctionReferenceKind::BoolCondition | FunctionReferenceKind::MatchCondition => {
                return None;
            }
        }
    }

    let mut modes = Vec::new();
    if deferred {
        modes.push("deferred");
    }
    if immediate {
        modes.push("immediate");
    }
    if blocking {
        modes.push("blocking");
    }
    (!modes.is_empty()).then_some(modes)
}

fn schema_code_action(
    params: &CodeActionParams,
    schema: &SchemaCodeActionDocument,
    documents: &[CodeActionDocument<'_>],
    plan: &recite_core::SchemaSourceEditPlan,
    title: String,
) -> Option<CodeActionOrCommand> {
    Some(CodeActionOrCommand::CodeAction(CodeAction {
        title,
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(params.context.diagnostics.clone()),
        edit: Some(schema_workspace_edit(schema, plan, documents)?),
        ..CodeAction::default()
    }))
}
