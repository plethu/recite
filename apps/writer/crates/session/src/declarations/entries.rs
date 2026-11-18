use crate::messages::{MsgId, text};

/// What the declarations sheet shows for one declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Detail {
    /// A condition's or effect's parameters in declaration order, as (name, author-facing type).
    Parameters(Vec<(String, String)>),
    /// A speaker's display name, a registry's values one per line, or a pointer to the source.
    Text(String),
}

pub fn entries(schema: &recite_core::schema::ProjectSchema) -> Vec<(String, String, Detail)> {
    let mut rows = Vec::new();
    for (name, condition) in &schema.conditions {
        rows.push((
            name.clone(),
            text(MsgId::WriterDeclarationCondition),
            Detail::Parameters(signature(&condition.params)),
        ));
    }
    for (name, effect) in &schema.effects {
        rows.push((
            name.clone(),
            text(MsgId::WriterDeclarationEffect),
            Detail::Parameters(signature(&effect.params)),
        ));
    }
    for (name, speaker) in &schema.speakers {
        rows.push((
            name.clone(),
            text(MsgId::WriterDeclarationSpeaker),
            Detail::Text(speaker.display_name.clone().unwrap_or_else(|| name.clone())),
        ));
    }
    for (name, registry) in &schema.registries {
        rows.push((
            name.clone(),
            text(MsgId::WriterDeclarationRegistry),
            Detail::Text(
                registry
                    .values
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
        ));
    }
    for (kind, names) in [
        ("Types", schema.types.keys().cloned().collect::<Vec<_>>()),
        (
            "Availability reasons",
            schema
                .availability_reasons
                .keys()
                .map(ToString::to_string)
                .collect(),
        ),
        (
            "Metadata domains",
            schema.metadata_domains.keys().cloned().collect(),
        ),
        ("Metadata", schema.metadata.keys().cloned().collect()),
        (
            "Projection queries",
            schema.projection_queries.keys().cloned().collect(),
        ),
        (
            "Presentation projectors",
            schema.presentation_projectors.keys().cloned().collect(),
        ),
        ("Markup", schema.markup.keys().cloned().collect()),
    ] {
        for name in names {
            rows.push((
                name,
                kind.into(),
                Detail::Text(text(MsgId::WriterDeclarationSourceDetails)),
            ));
        }
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    rows
}
fn signature(params: &[recite_core::schema::ParameterDefinition]) -> Vec<(String, String)> {
    params
        .iter()
        .map(|p| (p.name.clone(), parameter_type(&p.type_ref)))
        .collect()
}
fn parameter_type(value: &recite_core::schema::SchemaTypeRef) -> String {
    use recite_core::schema::SchemaTypeRef as T;
    match value {
        T::String => "Text".into(),
        T::Symbol => "Symbol".into(),
        T::Int => "Integer".into(),
        T::Float => "Number".into(),
        T::Bool => "True or false".into(),
        T::Speaker => "Speaker".into(),
        T::Enum(name) | T::Registry(name) => name.clone(),
        T::Array(inner) => format!("List of {}", parameter_type(inner)),
    }
}

pub fn kind_label(kind: &str) -> String {
    match kind {
        "condition" => text(MsgId::WriterDeclarationCondition),
        "effect" => text(MsgId::WriterDeclarationEffect),
        "speaker" => text(MsgId::WriterDeclarationSpeaker),
        "registry" => text(MsgId::WriterDeclarationRegistry),
        "type" => "Types".into(),
        "availability_reason" => "Availability reasons".into(),
        "metadata_domain" => "Metadata domains".into(),
        "metadata" => "Metadata".into(),
        "projection_query" => "Projection queries".into(),
        "presentation_projector" => "Presentation projectors".into(),
        "markup" => "Markup".into(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests;
