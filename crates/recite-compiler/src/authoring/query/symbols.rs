use recite_core::{DocumentKey, SourceId, SourcePosition, SourceSpan};

use super::super::snapshot::DocumentSnapshot;
use super::types::{SymbolIdentity, SymbolKind, SymbolLocation, SymbolQueryOptions, SymbolRole};

pub(super) fn symbol_locations(
    key: &DocumentKey,
    document: &DocumentSnapshot,
    options: SymbolQueryOptions,
) -> Vec<SymbolLocation> {
    let summary = document.summary();
    let mut locations = Vec::new();
    if options.include_declarations() {
        locations.extend(summary.blocks().iter().map(|block| SymbolLocation {
            document: key.clone(),
            identity: SymbolIdentity::Block(block.id().clone()),
            kind: SymbolKind::Block,
            role: SymbolRole::Definition,
            span: block.id_span().clone(),
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
            span: stable
                .source_id_span()
                .cloned()
                .unwrap_or_else(|| stable.insertion_span().clone()),
        })
    }));
    locations.extend(summary.metadata().iter().filter_map(|metadata| {
        let span = metadata
            .key_span()
            .cloned()
            .or_else(|| metadata.source_span().cloned())
            .or_else(|| metadata.value_span().cloned())?;
        Some(SymbolLocation {
            document: key.clone(),
            identity: SymbolIdentity::MetadataKey(metadata.key().to_owned()),
            kind: SymbolKind::Metadata,
            role: SymbolRole::Annotation,
            span,
        })
    }));
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
    locations.sort_by_key(|location| {
        let span = location.span();
        (
            span.start.line(),
            span.start.column(),
            span.end
                .map(|end| (end.line(), end.column()))
                .unwrap_or((span.start.line(), span.start.column())),
            location.kind as u8,
        )
    });
    locations
}

pub(super) fn contains(span: &SourceSpan, position: SourcePosition) -> bool {
    let start = (span.start.line(), span.start.column());
    let end = span
        .end
        .map(|end| (end.line(), end.column()))
        .unwrap_or(start);
    let point = (position.line(), position.column());
    start <= point && point <= end
}
