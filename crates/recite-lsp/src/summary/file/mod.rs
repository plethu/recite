mod identity;
mod items;

#[cfg(any(test, feature = "bench-support"))]
use std::path::Path;

use lsp_types::Uri;
use recite_compiler::{DocumentSnapshot, FunctionReferenceKind as AuthoringFunctionReferenceKind};
use recite_core::{SourceId, SourcePosition, is_valid_source_label};

#[cfg(any(test, feature = "bench-support"))]
use recite_core::Diagnostic;

pub(crate) use identity::{FileIdentity, OpenFileIdentity, SavedFileIdentity};
#[cfg(feature = "bench-support")]
pub(crate) use items::MetadataKeySummary;
pub(crate) use items::{
    BlockReferenceSummary, FileSummaryCompleteness, FunctionReferenceKind,
    FunctionReferenceSummary, MissingIdInsertion, MissingIdKind, MissingIdSummary, SpannedName,
};

#[derive(Clone, Debug)]
pub(crate) struct FileSummary {
    pub(crate) identity: FileIdentity,
    pub(crate) version: Option<i32>,
    #[cfg(any(test, feature = "bench-support"))]
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) completeness: FileSummaryCompleteness,
    pub(crate) blocks: Vec<SpannedName>,
    pub(crate) block_references: Vec<BlockReferenceSummary>,
    pub(crate) line_ids: Vec<SpannedName>,
    pub(crate) choice_ids: Vec<SpannedName>,
    pub(crate) missing_ids: Vec<MissingIdSummary>,
    #[cfg(feature = "bench-support")]
    pub(crate) metadata_keys: Vec<MetadataKeySummary>,
    pub(crate) condition_functions: Vec<FunctionReferenceSummary>,
    pub(crate) effect_functions: Vec<FunctionReferenceSummary>,
}

impl FileSummary {
    pub(crate) fn from_authoring(
        identity: FileIdentity,
        version: Option<i32>,
        document: &DocumentSnapshot,
    ) -> Self {
        let summary = document.summary();
        let participation = document.participation();
        let blocks = summary
            .blocks()
            .iter()
            .map(|block| SpannedName {
                name: block.id().as_str().to_owned(),
                span: block.span().clone(),
            })
            .collect();
        let block_references = summary
            .block_references()
            .iter()
            .map(|reference| BlockReferenceSummary {
                file: reference.file().map(ToOwned::to_owned),
                block_id: reference.block_id().as_str().to_owned(),
                span: reference
                    .block_id_span()
                    .cloned()
                    .unwrap_or_else(|| reference.span().clone()),
            })
            .collect();
        let mut line_ids = Vec::new();
        let mut choice_ids = Vec::new();
        let mut missing_ids = Vec::new();
        for stable_id in summary.stable_ids() {
            let (ids, kind) = match stable_id.kind() {
                recite_compiler::StableIdKind::Line => (&mut line_ids, MissingIdKind::Line),
                recite_compiler::StableIdKind::Choice => (&mut choice_ids, MissingIdKind::Choice),
                _ => continue,
            };
            match stable_id.source_id() {
                SourceId::Frozen { anchor, .. } => ids.push(SpannedName {
                    name: anchor.as_str().to_owned(),
                    span: stable_id.span().clone(),
                }),
                SourceId::Missing => missing_ids.push(MissingIdSummary {
                    kind,
                    label: None,
                    insertion: MissingIdInsertion::FullId,
                    span: stable_id.span().clone(),
                    insertion_position: insertion_position_after_marker(stable_id),
                }),
                SourceId::Draft { label } => missing_ids.push(MissingIdSummary {
                    kind,
                    label: Some(label.clone()),
                    insertion: MissingIdInsertion::AnchorOnly,
                    span: stable_id.span().clone(),
                    insertion_position: insertion_position(stable_id),
                }),
                SourceId::Malformed { raw } if is_valid_source_label(raw) => {
                    missing_ids.push(MissingIdSummary {
                        kind,
                        label: Some(raw.clone()),
                        insertion: MissingIdInsertion::AtAnchor,
                        span: stable_id.span().clone(),
                        insertion_position: insertion_position(stable_id),
                    });
                }
                SourceId::Malformed { .. } => {}
            }
        }
        #[cfg(feature = "bench-support")]
        let metadata_keys = summary
            .metadata()
            .iter()
            .map(|metadata| MetadataKeySummary {
                key: metadata.key().to_owned(),
                key_span: metadata.key_span().cloned(),
                entry_span: metadata.source_span().cloned(),
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
            version,
            #[cfg(any(test, feature = "bench-support"))]
            diagnostics: document.diagnostics().to_vec(),
            completeness: FileSummaryCompleteness {
                block_definitions: participation.block_definitions().is_complete(),
                block_references: participation.block_references().is_complete(),
                stable_ids: participation.stable_ids().is_complete(),
                metadata: participation.metadata().is_complete(),
                condition_functions: participation.condition_functions().is_complete(),
                effect_functions: participation.effect_functions().is_complete(),
                inline_markup: participation.inline_markup().is_complete(),
                recoverable_regions: !participation.ast_structure().is_complete(),
            },
            blocks,
            block_references,
            line_ids,
            choice_ids,
            missing_ids,
            #[cfg(feature = "bench-support")]
            metadata_keys,
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

fn insertion_position(stable_id: &recite_compiler::StableIdSummary) -> SourcePosition {
    stable_id
        .insertion_span()
        .map_or_else(|| stable_id.span().start, |span| span.start)
}

fn insertion_position_after_marker(stable_id: &recite_compiler::StableIdSummary) -> SourcePosition {
    SourcePosition::new(
        stable_id.span().start.line(),
        stable_id.span().start.column().saturating_add(1),
    )
    .unwrap_or(stable_id.span().start)
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
