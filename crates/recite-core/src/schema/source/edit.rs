use std::collections::{BTreeMap, VecDeque};

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
            if !is_manifest_name(&name) {
                return Err(SchemaSourceEditError::InvalidArgument(
                    "type name must be an identifier-like schema name".to_owned(),
                ));
            }
            if values.iter().any(|value| value.is_empty()) {
                return Err(SchemaSourceEditError::InvalidArgument(
                    "enum values must not be empty".to_owned(),
                ));
            }
            let type_table = declaration_table(&mut document, "types", &name)?;
            set_value_preserving_decor(type_table, "kind", "enum".to_owned());
            let item = type_table.entry("values").or_insert(toml_edit::Item::Value(
                toml_edit::Value::Array(toml_edit::Array::new()),
            ));
            let Some(array) = item.as_array_mut() else {
                return Err(SchemaSourceEditError::InvalidArgument(
                    "type values is not an array".to_owned(),
                ));
            };
            // Rebuild the array so comments attached to removed elements do
            // not migrate onto replacements. Retain element decoration only
            // for an exact value identity (and in source order for the rare
            // duplicate input); new and removed values get fresh decoration.
            let mut element_decorations = BTreeMap::new();
            let mut trailing_comments = Vec::new();
            let mut old_values = Vec::new();
            for (index, element) in array.iter().enumerate() {
                let Some(value) = element.as_str() else {
                    continue;
                };
                let (prefix, trailing_comment) = if index == 0 {
                    (
                        element
                            .decor()
                            .prefix()
                            .and_then(toml_edit::RawString::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                        None,
                    )
                } else {
                    split_inter_element_prefix(
                        element
                            .decor()
                            .prefix()
                            .and_then(toml_edit::RawString::as_str)
                            .unwrap_or_default(),
                    )
                };
                let mut decoration = element.decor().clone();
                decoration.clear();
                decoration.set_prefix(prefix);
                if let Some(suffix) = element.decor().suffix().cloned() {
                    decoration.set_suffix(suffix);
                }
                element_decorations
                    .entry(value.to_owned())
                    .or_insert_with(VecDeque::new)
                    .push_back((old_values.len(), decoration));
                old_values.push(value.to_owned());
                trailing_comments.push(None);
                if let Some(comment) = trailing_comment
                    && old_values.len() > 1
                {
                    trailing_comments[old_values.len() - 2] = Some(comment);
                }
            }
            let prefix = array.decor().prefix().cloned();
            let suffix = array.decor().suffix().cloned();
            let trailing = array.trailing().clone();
            let trailing_comma = array.trailing_comma();
            let mut replacement = toml_edit::Array::new();
            if let Some(prefix) = prefix {
                replacement.decor_mut().set_prefix(prefix);
            }
            if let Some(suffix) = suffix {
                replacement.decor_mut().set_suffix(suffix);
            }
            replacement.set_trailing(trailing);
            replacement.set_trailing_comma(trailing_comma);
            let mut new_old_indices = Vec::new();
            for value in values {
                let mut replacement_value = toml_edit::Value::from(value.clone());
                if let Some(decorations) = element_decorations.get_mut(&value)
                    && let Some((old_index, decoration)) = decorations.pop_front()
                {
                    new_old_indices.push(Some(old_index));
                    if let Some(prefix) = decoration.prefix().cloned() {
                        replacement_value.decor_mut().set_prefix(prefix);
                    }
                    if let Some(suffix) = decoration.suffix().cloned() {
                        replacement_value.decor_mut().set_suffix(suffix);
                    }
                } else {
                    new_old_indices.push(None);
                }
                replacement.push(replacement_value);
            }
            preserve_retained_trailing_comments(
                &mut replacement,
                &new_old_indices,
                &trailing_comments,
                &old_values,
            );
            *array = replacement;
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

#[derive(Clone, Debug)]
struct InterElementComment {
    line: String,
    indent: String,
}

/// `toml_edit` stores a comment after a comma in the next element's prefix.
/// Split that one-line trailing comment from the next element's own trivia so
/// it can remain attached to a retained preceding value when the next value
/// is removed.
fn split_inter_element_prefix(prefix: &str) -> (String, Option<InterElementComment>) {
    let Some(newline) = prefix.find('\n') else {
        return (prefix.to_owned(), None);
    };
    let first_line = &prefix[..newline];
    if !first_line.contains('#') {
        return (prefix.to_owned(), None);
    }
    let after_first_line = &prefix[newline + 1..];
    let indent_end = after_first_line.find('#').unwrap_or(after_first_line.len());
    (
        after_first_line.to_owned(),
        Some(InterElementComment {
            line: first_line.to_owned(),
            indent: after_first_line[..indent_end].to_owned(),
        }),
    )
}

fn preserve_retained_trailing_comments(
    replacement: &mut toml_edit::Array,
    new_old_indices: &[Option<usize>],
    trailing_comments: &[Option<InterElementComment>],
    old_values: &[String],
) {
    let last_old_retained = old_values
        .len()
        .checked_sub(1)
        .is_some_and(|last| new_old_indices.contains(&Some(last)));
    let mut trailing = replacement
        .trailing()
        .as_str()
        .unwrap_or_default()
        .to_owned();
    if !last_old_retained {
        trailing = remove_comment_text(&trailing);
    }

    let mut trailing_prefix = String::new();
    for (old_index, comment) in trailing_comments.iter().enumerate() {
        let Some(comment) = comment else {
            continue;
        };
        let Some(new_index) = new_old_indices
            .iter()
            .position(|index| *index == Some(old_index))
        else {
            continue;
        };
        let Some(next_index) = new_index.checked_add(1) else {
            continue;
        };
        if let Some(next) = replacement.get_mut(next_index) {
            let next_prefix = next
                .decor()
                .prefix()
                .and_then(toml_edit::RawString::as_str)
                .unwrap_or_default();
            let mut prefix = comment.line.clone();
            prefix.push('\n');
            if next_prefix.is_empty() {
                prefix.push_str(&comment.indent);
            } else {
                prefix.push_str(next_prefix);
            }
            next.decor_mut().set_prefix(prefix);
        } else {
            trailing_prefix.push_str(&comment.line);
            trailing_prefix.push('\n');
            trailing_prefix.push_str(&comment.indent);
        }
    }

    if !trailing_prefix.is_empty() {
        trailing_prefix.push_str(&trailing);
        trailing = trailing_prefix;
    }
    replacement.set_trailing(trailing);
}

fn remove_comment_text(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut in_comment = false;
    for character in value.chars() {
        if in_comment {
            if character == '\n' {
                in_comment = false;
                result.push(character);
            }
        } else if character == '#' {
            in_comment = true;
        } else {
            result.push(character);
        }
    }
    result
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

fn set_value_preserving_decor(table: &mut toml_edit::Table, key: &str, value: String) {
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
