mod identity;
mod items;

#[cfg(any(test, feature = "bench-support"))]
use std::path::Path;

use lsp_types::Uri;
use recite_compiler::{DocumentSnapshot, FunctionReferenceKind as AuthoringFunctionReferenceKind};

#[cfg(any(test, feature = "bench-support"))]
use recite_core::Diagnostic;

pub(crate) use identity::{FileIdentity, OpenFileIdentity, SavedFileIdentity};
#[cfg(any(test, feature = "bench-support"))]
pub(crate) use items::SpannedName;
pub(crate) use items::{FileSummaryCompleteness, FunctionReferenceKind, FunctionReferenceSummary};

#[derive(Clone, Debug)]
pub(crate) struct FileSummary {
    pub(crate) identity: FileIdentity,
    #[cfg(any(test, feature = "bench-support"))]
    pub(crate) version: Option<i32>,
    #[cfg(any(test, feature = "bench-support"))]
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) completeness: FileSummaryCompleteness,
    #[cfg(any(test, feature = "bench-support"))]
    pub(crate) blocks: Vec<SpannedName>,
    pub(crate) condition_functions: Vec<FunctionReferenceSummary>,
    pub(crate) effect_functions: Vec<FunctionReferenceSummary>,
}

impl FileSummary {
    pub(crate) fn from_authoring(
        identity: FileIdentity,
        _version: Option<i32>,
        document: &DocumentSnapshot,
    ) -> Self {
        let summary = document.summary();
        let participation = document.participation();
        #[cfg(any(test, feature = "bench-support"))]
        let blocks = summary
            .blocks()
            .iter()
            .map(|block| SpannedName {
                name: block.id().as_str().to_owned(),
                span: block.span().clone(),
            })
            .collect();
        let condition_functions = summary
            .condition_functions()
            .iter()
            .filter_map(function_reference)
            .collect();
        let effect_functions = summary
            .effect_functions()
            .iter()
            .filter_map(function_reference)
            .collect();
        Self {
            identity,
            #[cfg(any(test, feature = "bench-support"))]
            version: _version,
            #[cfg(any(test, feature = "bench-support"))]
            diagnostics: document.diagnostics().to_vec(),
            completeness: FileSummaryCompleteness {
                block_definitions: participation.block_definitions().is_complete(),
                metadata: participation.metadata().is_complete(),
                condition_functions: participation.condition_functions().is_complete(),
                effect_functions: participation.effect_functions().is_complete(),
                inline_markup: participation.inline_markup().is_complete(),
                recoverable_regions: !participation.ast_structure().is_complete(),
            },
            #[cfg(any(test, feature = "bench-support"))]
            blocks,
            condition_functions,
            effect_functions,
        }
    }

    pub(crate) fn uri(&self) -> &Uri {
        self.identity.uri()
    }

    #[cfg(any(test, feature = "bench-support"))]
    pub(crate) fn saved_path(&self) -> Option<&Path> {
        self.identity.saved_path()
    }

    pub(crate) fn project_relative_path(&self) -> Option<&str> {
        self.identity.project_relative_path()
    }
}

fn function_reference(
    function: &recite_compiler::FunctionReferenceSummary,
) -> Option<FunctionReferenceSummary> {
    Some(FunctionReferenceSummary {
        name: function.name().to_owned(),
        span: function.span().clone(),
        argument_count: function.argument_count(),
        kind: match function.kind() {
            AuthoringFunctionReferenceKind::BooleanCondition => {
                FunctionReferenceKind::BoolCondition
            }
            AuthoringFunctionReferenceKind::MatchCondition => FunctionReferenceKind::MatchCondition,
            AuthoringFunctionReferenceKind::DeferredEffect => FunctionReferenceKind::DeferredEffect,
            AuthoringFunctionReferenceKind::ImmediateEffect => {
                FunctionReferenceKind::ImmediateEffect
            }
            AuthoringFunctionReferenceKind::BlockingEffect => FunctionReferenceKind::BlockingEffect,
            _ => return None,
        },
    })
}
