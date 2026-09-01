use recite_core::{DocumentKey, SourceSpan};

use super::super::snapshot::AuthoringSnapshot;
use super::types::{
    ClauseKind, HoverInfo, QueryResult, SemanticFact, SymbolIdentity, SymbolKind, SymbolLocation,
    SymbolRole,
};

pub(super) fn clause_hover(key: &DocumentKey, kind: ClauseKind, span: SourceSpan) -> HoverInfo {
    HoverInfo {
        location: SymbolLocation {
            document: key.clone(),
            identity: SymbolIdentity::Clause(kind),
            kind: SymbolKind::Clause,
            role: SymbolRole::Annotation,
            span,
        },
        facts: vec![SemanticFact::Clause { kind }],
        metadata_value: None,
    }
}

impl AuthoringSnapshot {
    pub(super) fn availability_reason_hover(
        &self,
        key: &DocumentKey,
        value: String,
        token: SourceSpan,
    ) -> QueryResult<HoverInfo> {
        if !recite_parser::is_metadata_symbol(&value) {
            return QueryResult::NoMatch;
        }
        let Some(schema) = &self.schema else {
            return QueryResult::NoMatch;
        };
        let Some(definition) = schema.availability_reasons.get(value.as_str()) else {
            return QueryResult::NoMatch;
        };
        QueryResult::Ready(HoverInfo {
            location: SymbolLocation {
                document: key.clone(),
                identity: SymbolIdentity::Schema(value.clone()),
                kind: SymbolKind::Schema,
                role: SymbolRole::Annotation,
                span: token,
            },
            facts: vec![SemanticFact::AvailabilityReason {
                name: value,
                template: definition.template.clone(),
                parameters: definition.params.len(),
            }],
            metadata_value: None,
        })
    }
}
