use recite_core::{DocumentKey, ProjectSchema, SourcePosition, SourceSpan};

use super::super::snapshot::{AuthoringSnapshot, DocumentSnapshot};
use super::context;
use super::types::SemanticSymbolKind;

impl AuthoringSnapshot {
    pub(super) fn schema_symbol_at(
        &self,
        key: &DocumentKey,
        document: &DocumentSnapshot,
        position: SourcePosition,
    ) -> Option<(String, SourceSpan, SemanticSymbolKind)> {
        let schema = self.schema.as_deref()?;
        let (name, span) = context::token_at(key, document.source_text(), position)?;
        let kind = schema_symbol_kind(schema, &name)?;
        Some((name, span, kind))
    }
}

fn schema_symbol_kind(schema: &ProjectSchema, name: &str) -> Option<SemanticSymbolKind> {
    if schema.speakers.contains_key(name) {
        return Some(SemanticSymbolKind::Speaker);
    }
    if schema.registries.contains_key(name) {
        return Some(SemanticSymbolKind::Registry);
    }
    if schema.metadata_domains.contains_key(name) {
        return Some(SemanticSymbolKind::MetadataDomain);
    }
    if schema.metadata.contains_key(name) {
        return Some(SemanticSymbolKind::Metadata);
    }
    if schema.availability_reasons.contains_key(name) {
        return Some(SemanticSymbolKind::AvailabilityReason);
    }
    if schema.conditions.contains_key(name) {
        return Some(SemanticSymbolKind::Condition);
    }
    if schema.effects.contains_key(name) {
        return Some(SemanticSymbolKind::Effect);
    }
    if schema.projection_queries.contains_key(name) {
        return Some(SemanticSymbolKind::ProjectionQuery);
    }
    if let Some(projector) = schema.presentation_projectors.get(name) {
        return Some(SemanticSymbolKind::ProjectionProjector {
            inputs: projector.inputs.len(),
            queries: projector.queries.len(),
            outputs: projector.outputs.len(),
        });
    }
    if let Some((projector, output)) =
        schema
            .presentation_projectors
            .iter()
            .find_map(|(projector, definition)| {
                definition
                    .outputs
                    .get(name)
                    .map(|output| (projector, output))
            })
    {
        return Some(SemanticSymbolKind::ProjectionOutput {
            projector: projector.clone(),
            target: output.target.clone(),
            kind: output.kind.clone(),
        });
    }
    schema
        .presentation_projectors
        .values()
        .flat_map(|projector| projector.outputs.values())
        .find_map(|output| {
            output.label.as_ref().and_then(|label| {
                (label.template_id == name).then_some(SemanticSymbolKind::ProjectionLabel {
                    arguments: label.args.len(),
                })
            })
        })
}
