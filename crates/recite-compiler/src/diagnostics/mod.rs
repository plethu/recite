mod asset;
mod conditions;
mod effects;
mod ids;
mod markup;
mod metadata;
mod project;

#[cfg(test)]
mod tests;

use recite_core::{
    Diagnostic, DiagnosticArgumentValue, DiagnosticCode, DiagnosticPresentation,
    DiagnosticPresentationContract, DiagnosticPresentationId, DiagnosticRelatedPresentation,
    SchemaTypeRef, SourceSpan, auxiliary_contract_for, contract_for, explain_diagnostic_code,
};

pub(crate) use asset::*;
pub(crate) use conditions::*;
pub(crate) use effects::*;
pub(crate) use ids::*;
pub(crate) use markup::*;
pub(crate) use metadata::*;
pub(crate) use project::*;

pub(crate) fn display_schema_type_ref(type_ref: &SchemaTypeRef) -> String {
    match type_ref {
        SchemaTypeRef::String => "string".to_owned(),
        SchemaTypeRef::Symbol => "symbol".to_owned(),
        SchemaTypeRef::Int => "int".to_owned(),
        SchemaTypeRef::Float => "float".to_owned(),
        SchemaTypeRef::Bool => "bool".to_owned(),
        SchemaTypeRef::Speaker => "speaker".to_owned(),
        SchemaTypeRef::Enum(name) => format!("enum:{name}"),
        SchemaTypeRef::Registry(name) => format!("registry:{name}"),
        SchemaTypeRef::Array(inner) => format!("array:{}", display_schema_type_ref(inner)),
    }
}

pub(crate) fn string_argument(value: impl Into<String>) -> DiagnosticArgumentValue {
    DiagnosticArgumentValue::String(value.into())
}

pub(crate) fn integer_argument(value: usize) -> DiagnosticArgumentValue {
    DiagnosticArgumentValue::Integer(value as i64)
}

#[allow(
    clippy::expect_used,
    reason = "the central compiler registry is validated before producers run"
)]
pub(crate) fn diagnostic_contract(
    code: &DiagnosticCode,
    presentation_id: &'static str,
) -> &'static DiagnosticPresentationContract {
    contract_for(code, &DiagnosticPresentationId::new_static(presentation_id))
        .expect("compiler diagnostic presentation contract is registered")
}

#[allow(
    clippy::expect_used,
    reason = "compiler factories are paired with checked central contracts"
)]
pub(crate) fn compiler_diagnostic(
    contract: &'static DiagnosticPresentationContract,
    message: impl Into<String>,
    span: SourceSpan,
    arguments: impl IntoIterator<Item = (String, DiagnosticArgumentValue)>,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::error_from_contract(contract, message, span, arguments)
        .expect("compiler diagnostic arguments match their central contract");
    if let Some(explanation) = explain_diagnostic_code(&diagnostic.code) {
        diagnostic = diagnostic.with_explanation_presentation(explanation.presentation());
    }
    diagnostic
}

#[allow(
    clippy::expect_used,
    reason = "compiler factories are paired with checked auxiliary contracts"
)]
pub(crate) fn auxiliary_presentation(
    presentation_id: &'static str,
    arguments: impl IntoIterator<Item = (String, DiagnosticArgumentValue)>,
) -> DiagnosticPresentation {
    let contract = auxiliary_contract_for(&DiagnosticPresentationId::new_static(presentation_id))
        .expect("compiler diagnostic auxiliary contract is registered");
    contract
        .presentation(arguments)
        .expect("compiler diagnostic auxiliary arguments match their central contract")
}

pub(crate) fn related_presentation(
    span: SourceSpan,
    presentation_id: &'static str,
    arguments: impl IntoIterator<Item = (String, DiagnosticArgumentValue)>,
) -> DiagnosticRelatedPresentation {
    DiagnosticRelatedPresentation::new(span, auxiliary_presentation(presentation_id, arguments))
}
