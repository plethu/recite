use recite_compiler::authoring::{SchemaAction, SchemaCapability, SchemaSummary};
use std::collections::BTreeMap;

pub(super) struct DeclaredAction<'a> {
    pub(super) context: DeclarationContext<'a>,
    pub(super) action: &'a SchemaAction,
    pub(super) producer: Option<&'a recite_core::schema::ProducerIdentity>,
    pub(super) origin: Option<&'a recite_core::schema::ProducerOrigin>,
}

#[derive(Clone, Copy)]
pub(super) struct DeclarationContext<'a> {
    pub(super) kind: &'static str,
    pub(super) name: Option<&'a str>,
}

impl<'a> DeclarationContext<'a> {
    const fn schema() -> Self {
        Self {
            kind: "schema",
            name: None,
        }
    }

    fn named(kind: &'static str, name: &'a str) -> Self {
        Self {
            kind,
            name: Some(name),
        }
    }
}

pub(super) fn collect<'a>(summary: &'a SchemaSummary) -> Vec<DeclaredAction<'a>> {
    let mut collector = Collector::default();
    collector.add(
        DeclarationContext::schema(),
        summary.capability(),
        summary.ownership().producer(),
        None,
    );
    for declaration in summary.types() {
        collector.add(
            DeclarationContext::named("type", declaration.name()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
            declaration.provenance().origin(),
        );
    }
    for declaration in summary.registries() {
        collector.add(
            DeclarationContext::named("registry", declaration.name()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
            declaration.provenance().origin(),
        );
    }
    for declaration in summary.speakers() {
        collector.add(
            DeclarationContext::named("speaker", declaration.name()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
            declaration.provenance().origin(),
        );
    }
    for declaration in summary.conditions() {
        collector.add(
            DeclarationContext::named("condition", declaration.name()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
            declaration.provenance().origin(),
        );
    }
    for declaration in summary.availability_reasons() {
        collector.add(
            DeclarationContext::named("reason", declaration.id().as_str()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
            declaration.provenance().origin(),
        );
    }
    for declaration in summary.effects() {
        collector.add(
            DeclarationContext::named("effect", declaration.name()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
            declaration.provenance().origin(),
        );
    }
    for declaration in summary.metadata_domains() {
        collector.add(
            DeclarationContext::named("metadata-domain", declaration.name()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
            declaration.provenance().origin(),
        );
    }
    for declaration in summary.metadata() {
        collector.add(
            DeclarationContext::named("metadata", declaration.name()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
            declaration.provenance().origin(),
        );
    }
    for declaration in summary.projection_queries() {
        collector.add(
            DeclarationContext::named("projection-query", declaration.name()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
            declaration.provenance().origin(),
        );
    }
    for declaration in summary.presentation_projectors() {
        collector.add(
            DeclarationContext::named("projector", declaration.name()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
            declaration.provenance().origin(),
        );
    }
    for declaration in summary.markup() {
        collector.add(
            DeclarationContext::named("markup", declaration.name()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
            declaration.provenance().origin(),
        );
    }
    collector.declared
}

#[derive(Default)]
struct Collector<'a> {
    declared: Vec<DeclaredAction<'a>>,
    seen: BTreeMap<(&'static str, Option<&'a str>), Vec<&'a SchemaAction>>,
}

impl<'a> Collector<'a> {
    fn add(
        &mut self,
        context: DeclarationContext<'a>,
        capability: &'a SchemaCapability,
        producer: Option<&'a recite_core::schema::ProducerIdentity>,
        origin: Option<&'a recite_core::schema::ProducerOrigin>,
    ) {
        // Names may repeat across declaration kinds. Deduplicate only the same
        // context/action, retaining its first provenance and source order.
        let seen = self.seen.entry((context.kind, context.name)).or_default();
        for action in capability.actions() {
            if seen.contains(&action) {
                continue;
            }
            seen.push(action);
            self.declared.push(DeclaredAction {
                context,
                action,
                producer,
                origin,
            });
        }
    }
}
