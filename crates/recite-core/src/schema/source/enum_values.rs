use std::collections::{BTreeMap, VecDeque};

use super::toml::SchemaSourceEditError;
use crate::schema::manifest::validate::is_manifest_name;
use toml_edit::DocumentMut;

pub(super) fn set_enum_values(
    document: &mut DocumentMut,
    name: &str,
    values: Vec<String>,
) -> Result<(), SchemaSourceEditError> {
    if !is_manifest_name(name) {
        return Err(SchemaSourceEditError::InvalidArgument(
            "type name must be an identifier-like schema name".to_owned(),
        ));
    }
    if values.iter().any(String::is_empty) {
        return Err(SchemaSourceEditError::InvalidArgument(
            "enum values must not be empty".to_owned(),
        ));
    }
    let root = document.as_table_mut();
    let types = root
        .entry("types")
        .or_insert(toml_edit::Item::Table(toml_edit::Table::new()));
    let types = types.as_table_mut().ok_or_else(|| {
        SchemaSourceEditError::InvalidArgument("'types' is not a TOML table".to_owned())
    })?;
    let item = types
        .entry(name)
        .or_insert(toml_edit::Item::Table(toml_edit::Table::new()));
    let type_table = item.as_table_mut().ok_or_else(|| {
        SchemaSourceEditError::InvalidArgument(format!("'types.{name}' is not a TOML table"))
    })?;
    super::edit::set_value_preserving_decor(type_table, "kind", "enum".to_owned());
    let item =
        type_table
            .entry("values")
            .or_insert(toml_edit::Item::Value(toml_edit::Value::Array(
                toml_edit::Array::new(),
            )));
    let Some(array) = item.as_array_mut() else {
        return Err(SchemaSourceEditError::InvalidArgument(
            "type values is not an array".to_owned(),
        ));
    };
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
    Ok(())
}

#[derive(Clone, Debug)]
struct InterElementComment {
    line: String,
    indent: String,
}

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
    array: &mut toml_edit::Array,
    indices: &[Option<usize>],
    comments: &[Option<InterElementComment>],
    old_values: &[String],
) {
    let last_old_retained = old_values
        .len()
        .checked_sub(1)
        .is_some_and(|last| indices.contains(&Some(last)));
    let mut trailing = array.trailing().as_str().unwrap_or_default().to_owned();
    if !last_old_retained {
        trailing = remove_comment_text(&trailing);
    }
    let mut trailing_prefix = String::new();
    for (old_index, comment) in comments.iter().enumerate() {
        let Some(comment) = comment else { continue };
        let Some(new_index) = indices.iter().position(|index| *index == Some(old_index)) else {
            continue;
        };
        let Some(next) = new_index
            .checked_add(1)
            .and_then(|index| array.get_mut(index))
        else {
            trailing_prefix.push_str(&comment.line);
            trailing_prefix.push('\n');
            trailing_prefix.push_str(&comment.indent);
            continue;
        };
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
    }
    if !trailing_prefix.is_empty() {
        trailing_prefix.push_str(&trailing);
        trailing = trailing_prefix;
    }
    array.set_trailing(trailing);
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
