use std::collections::{BTreeMap, BTreeSet};

use recite_core::{ScalarValue, ast::InterpolationType, compiled::CompiledInterpolationBinding};

use crate::DialogueError;
use crate::locale::InterpolationValueProvider;

#[derive(Clone, Copy)]
pub(super) struct ResolvedCount<'a> {
    pub(super) binding_name: &'a str,
    pub(super) value_name: &'a str,
    pub(super) value: i64,
}

/// Render a template with one scanner shared by ordinary text and reason text.
/// Escaped braces are consumed here, and resolver values are never rescanned.
pub(in crate::traversal) fn render_template<T: AsRef<str>>(
    template: &str,
    mut resolve: impl FnMut(&str) -> Result<Option<T>, DialogueError>,
) -> Result<String, DialogueError> {
    let mut output = String::with_capacity(template.len());
    let mut cursor = 0;
    while cursor < template.len() {
        let remaining = &template[cursor..];
        let literal_end = remaining.find(['\\', '{']).unwrap_or(remaining.len());
        output.push_str(&remaining[..literal_end]);
        cursor += literal_end;
        let remaining = &remaining[literal_end..];
        if remaining.is_empty() {
            break;
        }
        if remaining.starts_with('\\') {
            let next = remaining.get(1..).and_then(|tail| tail.chars().next());
            if let Some(next @ ('{' | '}')) = next {
                output.push(next);
                cursor += 1 + next.len_utf8();
                continue;
            }
        }
        if remaining.starts_with('{') {
            let Some(close) = remaining.find('}') else {
                return Err(DialogueError::InvalidInterpolationSyntax {
                    reason: "unterminated placeholder".to_owned(),
                });
            };
            let name = &remaining[1..close];
            let Some(value) = resolve(name)? else {
                return Err(DialogueError::InvalidInterpolationSyntax {
                    reason: format!("placeholder `{name}` has no renderable value"),
                });
            };
            output.push_str(value.as_ref());
            cursor += close + 1;
            continue;
        }
        output.push('\\');
        cursor += 1;
    }
    Ok(output)
}

pub(in crate::traversal) fn render_bound_template(
    template: &str,
    bindings: &[CompiledInterpolationBinding],
    values: Option<&dyn InterpolationValueProvider>,
) -> Result<String, DialogueError> {
    render_bound_template_inner(template, bindings, values, None, None)
}

pub(super) fn render_selected_bound_template(
    template: &str,
    bindings: &[CompiledInterpolationBinding],
    values: Option<&dyn InterpolationValueProvider>,
    resolved_count: Option<ResolvedCount<'_>>,
) -> Result<String, DialogueError> {
    let placeholders = recite_core::extract_placeholder_names(template).map_err(|error| {
        DialogueError::InvalidInterpolationSyntax {
            reason: error.message().to_owned(),
        }
    })?;
    render_bound_template_inner(
        template,
        bindings,
        values,
        Some(&placeholders),
        resolved_count,
    )
}

fn render_bound_template_inner(
    template: &str,
    bindings: &[CompiledInterpolationBinding],
    values: Option<&dyn InterpolationValueProvider>,
    placeholders: Option<&BTreeSet<String>>,
    resolved_count: Option<ResolvedCount<'_>>,
) -> Result<String, DialogueError> {
    let mut resolved = BTreeMap::new();
    for binding in bindings {
        if placeholders.is_some_and(|names| !names.contains(&binding.name)) {
            continue;
        }
        let value = if let Some(resolved_count) = resolved_count.filter(|resolved| {
            resolved.binding_name == binding.name && resolved.value_name == binding.value
        }) {
            ScalarValue::Integer(resolved_count.value)
        } else {
            let Some(values) = values else {
                return Err(DialogueError::MissingInterpolationValue {
                    name: binding.value.clone(),
                });
            };
            values
                .lookup_value(&binding.value)
                .map_err(|error| DialogueError::InterpolationValueFailed {
                    name: binding.value.clone(),
                    reason: error.reason().to_owned(),
                })?
                .ok_or_else(|| DialogueError::MissingInterpolationValue {
                    name: binding.value.clone(),
                })?
        };
        if !matches_type(&value, binding.value_type) {
            return Err(DialogueError::InterpolationValueFailed {
                name: binding.value.clone(),
                reason: format!(
                    "expected {}, got {}",
                    type_name(binding.value_type),
                    value_kind(&value)
                ),
            });
        }
        resolved.insert(binding.name.as_str(), scalar_text(value));
    }
    render_template(template, |name| Ok(resolved.get(name).map(String::as_str)))
}

fn matches_type(value: &ScalarValue, value_type: InterpolationType) -> bool {
    matches!(
        (value, value_type),
        (ScalarValue::String(_), InterpolationType::String)
            | (ScalarValue::Integer(_), InterpolationType::Integer)
            | (ScalarValue::Float(_), InterpolationType::Float)
            | (ScalarValue::Boolean(_), InterpolationType::Boolean)
    )
}

fn type_name(value_type: InterpolationType) -> &'static str {
    match value_type {
        InterpolationType::String => "string",
        InterpolationType::Integer => "int",
        InterpolationType::Float => "float",
        InterpolationType::Boolean => "bool",
    }
}

pub(super) fn value_kind(value: &ScalarValue) -> &'static str {
    match value {
        ScalarValue::String(_) => "string",
        ScalarValue::Integer(_) => "int",
        ScalarValue::Float(_) => "float",
        ScalarValue::Boolean(_) => "bool",
    }
}

fn scalar_text(value: ScalarValue) -> String {
    match value {
        ScalarValue::String(value) => value,
        ScalarValue::Integer(value) => value.to_string(),
        ScalarValue::Float(value) => value.to_string(),
        ScalarValue::Boolean(value) => value.to_string(),
    }
}
