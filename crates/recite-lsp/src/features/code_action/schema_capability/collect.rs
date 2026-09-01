use recite_compiler::{SchemaAction, SchemaCapability, SchemaSummary};

pub(super) struct DeclaredAction<'a> {
    pub(super) context: DeclarationContext,
    pub(super) action: &'a SchemaAction,
    pub(super) producer: Option<&'a recite_core::ProducerIdentity>,
    pub(super) origin: Option<&'a recite_core::ProducerOrigin>,
}

#[derive(Clone)]
pub(super) struct DeclarationContext {
    pub(super) kind: &'static str,
    pub(super) name: Option<String>,
}

impl DeclarationContext {
    const fn schema() -> Self {
        Self {
            kind: "schema",
            name: None,
        }
    }

    fn named(kind: &'static str, name: impl Into<String>) -> Self {
        Self {
            kind,
            name: Some(name.into()),
        }
    }
}

pub(super) fn collect<'a>(summary: &'a SchemaSummary) -> Vec<DeclaredAction<'a>> {
    let mut declared = Vec::new();
    add_capability(
        &mut declared,
        DeclarationContext::schema(),
        summary.capability(),
        summary.ownership().producer(),
        None,
    );
    for declaration in summary.types() {
        add_capability(
            &mut declared,
            DeclarationContext::named("type", declaration.name()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
            declaration.provenance().origin(),
        );
    }
    for declaration in summary.registries() {
        add_capability(
            &mut declared,
            DeclarationContext::named("registry", declaration.name()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
            declaration.provenance().origin(),
        );
    }
    for declaration in summary.speakers() {
        add_capability(
            &mut declared,
            DeclarationContext::named("speaker", declaration.name()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
            declaration.provenance().origin(),
        );
    }
    for declaration in summary.conditions() {
        add_capability(
            &mut declared,
            DeclarationContext::named("condition", declaration.name()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
            declaration.provenance().origin(),
        );
    }
    for declaration in summary.availability_reasons() {
        add_capability(
            &mut declared,
            DeclarationContext::named("reason", declaration.id().as_str()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
            declaration.provenance().origin(),
        );
    }
    for declaration in summary.effects() {
        add_capability(
            &mut declared,
            DeclarationContext::named("effect", declaration.name()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
            declaration.provenance().origin(),
        );
    }
    for declaration in summary.metadata_domains() {
        add_capability(
            &mut declared,
            DeclarationContext::named("metadata-domain", declaration.name()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
            declaration.provenance().origin(),
        );
    }
    for declaration in summary.metadata() {
        add_capability(
            &mut declared,
            DeclarationContext::named("metadata", declaration.name()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
            declaration.provenance().origin(),
        );
    }
    for declaration in summary.projection_queries() {
        add_capability(
            &mut declared,
            DeclarationContext::named("projection-query", declaration.name()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
            declaration.provenance().origin(),
        );
    }
    for declaration in summary.presentation_projectors() {
        add_capability(
            &mut declared,
            DeclarationContext::named("projector", declaration.name()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
            declaration.provenance().origin(),
        );
    }
    for declaration in summary.markup() {
        add_capability(
            &mut declared,
            DeclarationContext::named("markup", declaration.name()),
            declaration.capability(),
            declaration.provenance().ownership().producer(),
            declaration.provenance().origin(),
        );
    }
    declared
}

fn add_capability<'a>(
    declared: &mut Vec<DeclaredAction<'a>>,
    context: DeclarationContext,
    capability: &'a SchemaCapability,
    producer: Option<&'a recite_core::ProducerIdentity>,
    origin: Option<&'a recite_core::ProducerOrigin>,
) {
    for action in capability.actions() {
        if declared.iter().any(|candidate| {
            candidate.context.kind == context.kind
                && candidate.context.name == context.name
                && candidate.action == action
        }) {
            continue;
        }
        declared.push(DeclaredAction {
            context: context.clone(),
            action,
            producer,
            origin,
        });
    }
}
