use super::super::snapshot::DocumentSnapshot;
use super::types::{SymbolIdentity, SymbolKind, SymbolLocation, SymbolQueryOptions, SymbolRole};
use recite_core::{DocumentKey, SourceId, SourcePosition, SourceSpan};

pub(super) fn contains(span: &SourceSpan, position: SourcePosition) -> bool {
    if span.end.is_none() {
        return false;
    }
    let start = (span.start.line(), span.start.column());
    let end = span
        .end
        .map(|end| (end.line(), end.column()))
        .unwrap_or(start);
    let point = (position.line(), position.column());
    start <= point && point <= end
}

pub(super) fn symbol_locations(
    key: &DocumentKey,
    document: &DocumentSnapshot,
    options: SymbolQueryOptions,
) -> Vec<SymbolLocation> {
    let summary = document.summary();
    let mut locations = Vec::new();
    if options.include_declarations() {
        locations.extend(summary.blocks().iter().filter_map(|block| {
            Some(SymbolLocation {
                document: key.clone(),
                identity: SymbolIdentity::Block(block.id().clone()),
                kind: SymbolKind::Block,
                role: SymbolRole::Definition,
                span: block.id_span()?.clone(),
            })
        }));
    }
    locations.extend(summary.block_references().iter().map(|reference| {
        SymbolLocation {
            document: key.clone(),
            identity: SymbolIdentity::Block(reference.block_id().clone()),
            kind: SymbolKind::BlockReference,
            role: SymbolRole::Reference,
            span: reference
                .block_id_span()
                .cloned()
                .unwrap_or_else(|| reference.span().clone()),
        }
    }));
    locations.extend(summary.stable_ids().iter().filter_map(|stable| {
        if !options.include_declarations() && matches!(stable.source_id(), SourceId::Frozen { .. })
        {
            return None;
        }
        Some(SymbolLocation {
            document: key.clone(),
            identity: SymbolIdentity::Source(stable.source_id().clone()),
            kind: SymbolKind::StableId,
            role: if matches!(stable.source_id(), SourceId::Frozen { .. }) {
                SymbolRole::Definition
            } else {
                SymbolRole::Annotation
            },
            span: stable.source_id_span()?.clone(),
        })
    }));
    locations.extend(summary.metadata().iter().filter_map(|metadata| {
        Some(SymbolLocation {
            document: key.clone(),
            identity: SymbolIdentity::MetadataKey(metadata.key().to_owned()),
            kind: SymbolKind::Metadata,
            role: SymbolRole::Annotation,
            span: metadata.key_span()?.clone(),
        })
    }));
    for metadata in summary.metadata() {
        let element_spans = metadata.value_element_spans();
        if element_spans.is_empty() {
            if let Some(span) = metadata.value_span() {
                locations.push(SymbolLocation {
                    document: key.clone(),
                    identity: SymbolIdentity::MetadataKey(metadata.key().to_owned()),
                    kind: SymbolKind::Metadata,
                    role: SymbolRole::Annotation,
                    span: span.clone(),
                });
            }
        } else {
            locations.extend(element_spans.iter().cloned().map(|span| SymbolLocation {
                document: key.clone(),
                identity: SymbolIdentity::MetadataKey(metadata.key().to_owned()),
                kind: SymbolKind::Metadata,
                role: SymbolRole::Annotation,
                span,
            }));
        }
    }
    locations.extend(
        summary
            .condition_functions()
            .iter()
            .map(|function| SymbolLocation {
                document: key.clone(),
                identity: SymbolIdentity::Function(function.name().to_owned()),
                kind: SymbolKind::ConditionFunction,
                role: SymbolRole::Invocation,
                span: function.span().clone(),
            }),
    );
    locations.extend(
        summary
            .effect_functions()
            .iter()
            .map(|function| SymbolLocation {
                document: key.clone(),
                identity: SymbolIdentity::Function(function.name().to_owned()),
                kind: SymbolKind::EffectFunction,
                role: SymbolRole::Invocation,
                span: function.span().clone(),
            }),
    );
    locations.sort_by(|left, right| {
        let left_span = left.span();
        let right_span = right.span();
        left_span
            .start
            .line()
            .cmp(&right_span.start.line())
            .then_with(|| left_span.start.column().cmp(&right_span.start.column()))
            .then_with(|| {
                let left_end = left_span
                    .end
                    .map(|end| (end.line(), end.column()))
                    .unwrap_or((left_span.start.line(), left_span.start.column()));
                let right_end = right_span
                    .end
                    .map(|end| (end.line(), end.column()))
                    .unwrap_or((right_span.start.line(), right_span.start.column()));
                left_end.cmp(&right_end)
            })
            .then_with(|| symbol_kind_order(left.kind()).cmp(&symbol_kind_order(right.kind())))
            .then_with(|| identity_order(left.identity()).cmp(&identity_order(right.identity())))
    });
    locations
}
fn symbol_kind_order(kind: SymbolKind) -> u8 {
    match kind {
        SymbolKind::Block => 0,
        SymbolKind::BlockReference => 1,
        SymbolKind::StableId => 2,
        SymbolKind::Metadata => 3,
        SymbolKind::ConditionFunction => 4,
        SymbolKind::EffectFunction => 5,
        SymbolKind::Schema => 6,
        SymbolKind::Clause => 7,
    }
}
fn identity_order(left: &SymbolIdentity) -> (u8, &str, &str) {
    match left {
        SymbolIdentity::Block(id) => (0, id.as_str(), ""),
        SymbolIdentity::Source(SourceId::Missing) => (1, "", ""),
        SymbolIdentity::Source(SourceId::Draft { label }) => (2, label, ""),
        SymbolIdentity::Source(SourceId::Frozen { label, anchor }) => (3, label, anchor.as_str()),
        SymbolIdentity::Source(SourceId::Malformed { raw }) => (4, raw, ""),
        SymbolIdentity::MetadataKey(key) => (5, key, ""),
        SymbolIdentity::Function(name) => (6, name, ""),
        SymbolIdentity::Schema(name) => (7, name, ""),
        SymbolIdentity::Clause(kind) => (8, clause_name(*kind), ""),
    }
}

fn clause_name(kind: super::types::ClauseKind) -> &'static str {
    match kind {
        super::types::ClauseKind::Requires => "requires",
        super::types::ClauseKind::If => "if",
    }
}
