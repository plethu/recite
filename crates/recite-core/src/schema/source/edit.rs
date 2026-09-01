use super::declarations::apply_declaration_edit;
use super::toml::{
    SchemaDeclarationKind, SchemaSource, SchemaSourceEdit, SchemaSourceEditError,
    load_schema_source_str,
};
use crate::schema::manifest::validate::is_manifest_name;
use toml_edit::DocumentMut;

pub(super) fn apply_edit(
    source: &mut SchemaSource,
    edit: SchemaSourceEdit,
) -> Result<(), SchemaSourceEditError> {
    let mut document = source.document.clone();
    match edit {
        SchemaSourceEdit::SetProducerId(id) => {
            if id.trim().is_empty() {
                return Err(SchemaSourceEditError::InvalidArgument(
                    "producer id must not be empty".to_owned(),
                ));
            }
            let producer = root_table(&mut document, "producer")?;
            set_value_preserving_decor(producer, "id", id);
        }
        SchemaSourceEdit::SetEnumValues { name, values } => {
            super::enum_values::set_enum_values(&mut document, &name, values)?;
        }
        edit @ (SchemaSourceEdit::AddCondition { .. }
        | SchemaSourceEdit::AddEffect { .. }
        | SchemaSourceEdit::AddAvailabilityReason { .. }) => {
            apply_declaration_edit(&mut document, edit)?;
        }
        SchemaSourceEdit::SetSpeakerDisplayName { name, display_name } => {
            if !is_manifest_name(&name) {
                return Err(SchemaSourceEditError::InvalidArgument(
                    "speaker name must be an identifier-like schema name".to_owned(),
                ));
            }
            let speaker = declaration_table(&mut document, "speakers", &name)?;
            match display_name {
                Some(display_name) => {
                    if display_name.trim().is_empty() {
                        return Err(SchemaSourceEditError::InvalidArgument(
                            "speaker display name must not be empty".to_owned(),
                        ));
                    }
                    set_value_preserving_decor(speaker, "display_name", display_name);
                }
                None => {
                    speaker.remove("display_name");
                }
            }
        }
        SchemaSourceEdit::RemoveDeclaration { kind, name } => {
            if !is_manifest_name(&name) {
                return Err(SchemaSourceEditError::InvalidArgument(
                    "declaration name must be an identifier-like schema name".to_owned(),
                ));
            }
            let section = declaration_section(kind);
            let root = document.as_table_mut();
            let Some(section) = root.get_mut(section).and_then(|item| item.as_table_mut()) else {
                return Err(SchemaSourceEditError::InvalidArgument(format!(
                    "declaration section '{section}' does not exist"
                )));
            };
            if section.remove(&name).is_none() {
                return Err(SchemaSourceEditError::InvalidArgument(format!(
                    "declaration '{name}' does not exist"
                )));
            }
        }
    }

    let text = super::toml::apply_source_layout_policy(document.to_string(), &source.source_text);
    let report = load_schema_source_str(source.file.clone(), &text);
    if !report.diagnostics.is_empty() {
        return Err(SchemaSourceEditError::Diagnostics(report.diagnostics));
    }
    let Some(updated) = report.source else {
        return Err(SchemaSourceEditError::InvalidArgument(
            "typed edit produced no schema".to_owned(),
        ));
    };
    *source = updated;
    Ok(())
}

pub(super) fn new_declaration<'a>(
    document: &'a mut DocumentMut,
    section: &str,
    name: &str,
) -> Result<&'a mut toml_edit::Table, SchemaSourceEditError> {
    if !is_manifest_name(name) {
        return Err(SchemaSourceEditError::InvalidArgument(
            "declaration name must be an identifier-like schema name".to_owned(),
        ));
    }
    let root = document.as_table_mut();
    let section_item = root
        .entry(section)
        .or_insert(toml_edit::Item::Table(toml_edit::Table::new()));
    let section_table = section_table_for_edit(section_item, section)?;
    if section_table.contains_key(name) {
        return Err(SchemaSourceEditError::InvalidArgument(format!(
            "declaration '{name}' already exists"
        )));
    }
    section_table.insert(name, toml_edit::Item::Table(toml_edit::Table::new()));
    section_table
        .get_mut(name)
        .and_then(toml_edit::Item::as_table_mut)
        .ok_or_else(|| {
            SchemaSourceEditError::InvalidArgument("new declaration is not a table".to_owned())
        })
}

fn section_table_for_edit<'a>(
    item: &'a mut toml_edit::Item,
    section: &str,
) -> Result<&'a mut toml_edit::Table, SchemaSourceEditError> {
    if item.is_table() {
        return item.as_table_mut().ok_or_else(|| {
            SchemaSourceEditError::InvalidArgument(format!("'{section}' is not a TOML table"))
        });
    }
    let Some(inline) = item.as_inline_table() else {
        return Err(SchemaSourceEditError::InvalidArgument(format!(
            "'{section}' is not a TOML table"
        )));
    };
    if !inline.is_empty() {
        return Err(SchemaSourceEditError::InvalidArgument(format!(
            "'{section}' is not a TOML table"
        )));
    }
    let decor = item.as_value().map(|value| value.decor().clone());
    let mut table = inline.clone().into_table();
    if let Some(decor) = decor {
        if let Some(prefix) = decor.prefix().cloned() {
            table.decor_mut().set_prefix(prefix);
        }
        if let Some(suffix) = decor.suffix().cloned() {
            table.decor_mut().set_suffix(suffix);
        }
    }
    *item = toml_edit::Item::Table(table);
    item.as_table_mut().ok_or_else(|| {
        SchemaSourceEditError::InvalidArgument(format!("'{section}' is not a TOML table"))
    })
}

pub(super) fn set_value_preserving_decor(table: &mut toml_edit::Table, key: &str, value: String) {
    if let Some(item) = table.get_mut(key)
        && let Some(old) = item.as_value()
    {
        let prefix = old.decor().prefix().cloned();
        let suffix = old.decor().suffix().cloned();
        let mut replacement = toml_edit::value(value);
        if let Some(replacement) = replacement.as_value_mut() {
            if let Some(prefix) = prefix {
                replacement.decor_mut().set_prefix(prefix);
            }
            if let Some(suffix) = suffix {
                replacement.decor_mut().set_suffix(suffix);
            }
        }
        *item = replacement;
    } else {
        table.insert(key, toml_edit::value(value));
    }
}

fn root_table<'a>(
    document: &'a mut DocumentMut,
    section: &str,
) -> Result<&'a mut toml_edit::Table, SchemaSourceEditError> {
    let root = document.as_table_mut();
    let item = root
        .entry(section)
        .or_insert(toml_edit::Item::Table(toml_edit::Table::new()));
    item.as_table_mut().ok_or_else(|| {
        SchemaSourceEditError::InvalidArgument(format!("'{section}' is not a TOML table"))
    })
}

fn declaration_table<'a>(
    document: &'a mut DocumentMut,
    section: &str,
    name: &str,
) -> Result<&'a mut toml_edit::Table, SchemaSourceEditError> {
    let section_table = root_table(document, section)?;
    let item = section_table
        .entry(name)
        .or_insert(toml_edit::Item::Table(toml_edit::Table::new()));
    item.as_table_mut().ok_or_else(|| {
        SchemaSourceEditError::InvalidArgument(format!("'{section}.{name}' is not a TOML table"))
    })
}

fn declaration_section(kind: SchemaDeclarationKind) -> &'static str {
    match kind {
        SchemaDeclarationKind::Type => "types",
        SchemaDeclarationKind::Registry => "registries",
        SchemaDeclarationKind::Speaker => "speakers",
        SchemaDeclarationKind::Condition => "conditions",
        SchemaDeclarationKind::AvailabilityReason => "availability_reasons",
        SchemaDeclarationKind::Effect => "effects",
        SchemaDeclarationKind::MetadataDomain => "metadata_domains",
        SchemaDeclarationKind::Metadata => "metadata",
        SchemaDeclarationKind::ProjectionQuery => "projection_queries",
        SchemaDeclarationKind::PresentationProjector => "presentation_projectors",
        SchemaDeclarationKind::Markup => "markup",
    }
}
