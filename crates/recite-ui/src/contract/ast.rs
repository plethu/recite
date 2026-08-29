use std::collections::{BTreeMap, BTreeSet};

use fluent_syntax::ast::{Expression, InlineExpression, Pattern, PatternElement};

use crate::{Client, ClientSpec, ResourceSpec, UiArgType};

use super::ContractIssue;

#[allow(clippy::expect_used)]
pub(super) fn argument_contract() -> BTreeMap<String, BTreeMap<String, UiArgType>> {
    #[derive(serde::Deserialize)]
    struct Manifest {
        arguments: BTreeMap<String, BTreeMap<String, UiArgTypeName>>,
    }
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "lowercase")]
    enum UiArgTypeName {
        String,
        Integer,
        Float,
        Boolean,
    }
    let manifest: Manifest = toml::from_str(include_str!("../../resources/arguments.toml"))
        .expect("embedded UI argument contract is valid");
    manifest
        .arguments
        .into_iter()
        .map(|(id, args)| {
            (
                id,
                args.into_iter()
                    .map(|(name, kind)| {
                        let kind = match kind {
                            UiArgTypeName::String => UiArgType::String,
                            UiArgTypeName::Integer => UiArgType::Integer,
                            UiArgTypeName::Float => UiArgType::Float,
                            UiArgTypeName::Boolean => UiArgType::Boolean,
                        };
                        (name, kind)
                    })
                    .collect(),
            )
        })
        .collect()
}

pub(super) fn collect_pattern(
    pattern: &Pattern<&str>,
    variables: &mut BTreeMap<String, UiArgType>,
    id: &str,
    issues: &mut BTreeSet<ContractIssue>,
) {
    for element in &pattern.elements {
        let PatternElement::Placeable { expression } = element else {
            continue;
        };
        collect_expression(expression, variables, id, issues);
    }
}

fn collect_expression(
    expression: &Expression<&str>,
    variables: &mut BTreeMap<String, UiArgType>,
    id: &str,
    issues: &mut BTreeSet<ContractIssue>,
) {
    match expression {
        Expression::Inline(inline) => collect_inline(inline, variables, id, issues),
        Expression::Select { selector, variants } => {
            collect_inline(selector, variables, id, issues);
            let mut variant_variables = Vec::with_capacity(variants.len());
            for variant in variants {
                let mut variables_in_variant = BTreeMap::new();
                collect_pattern(&variant.value, &mut variables_in_variant, id, issues);
                variant_variables.push(variables_in_variant);
            }
            if let Some(first) = variant_variables.first() {
                let mut all_names = first.keys().cloned().collect::<BTreeSet<_>>();
                for variables_in_variant in &variant_variables[1..] {
                    all_names.extend(variables_in_variant.keys().cloned());
                }
                for name in all_names {
                    if variant_variables
                        .iter()
                        .any(|variables_in_variant| !variables_in_variant.contains_key(&name))
                    {
                        issues.insert(ContractIssue::SelectorArgumentMismatch {
                            id: id.to_owned(),
                            name,
                        });
                    }
                }
                for variables_in_variant in variant_variables {
                    variables.extend(variables_in_variant);
                }
            }
        }
    }
}

fn collect_inline(
    inline: &InlineExpression<&str>,
    variables: &mut BTreeMap<String, UiArgType>,
    id: &str,
    issues: &mut BTreeSet<ContractIssue>,
) {
    match inline {
        InlineExpression::VariableReference { id } => {
            variables
                .entry(id.name.to_owned())
                .or_insert(UiArgType::String);
        }
        InlineExpression::Placeable { expression } => {
            collect_expression(expression, variables, id, issues);
        }
        InlineExpression::FunctionReference { arguments, .. } => {
            for positional in &arguments.positional {
                collect_inline(positional, variables, id, issues);
            }
            for named in &arguments.named {
                collect_inline(&named.value, variables, id, issues);
            }
        }
        InlineExpression::StringLiteral { .. }
        | InlineExpression::NumberLiteral { .. }
        | InlineExpression::MessageReference { .. }
        | InlineExpression::TermReference { .. } => {}
    }
}

pub(super) fn check_arguments(
    spec: &ResourceSpec,
    id: &str,
    actual: &BTreeMap<String, UiArgType>,
    issues: &mut BTreeSet<ContractIssue>,
) {
    for name in spec.arguments.keys() {
        if !actual.contains_key(name) {
            issues.insert(ContractIssue::MissingArgument {
                id: id.to_owned(),
                name: name.clone(),
            });
        }
    }
    for (name, actual_kind) in actual {
        let Some(expected_kind) = spec.arguments.get(name) else {
            issues.insert(ContractIssue::ExtraArgument {
                id: id.to_owned(),
                name: name.clone(),
            });
            continue;
        };
        // Fluent variables are deliberately untyped in the resource syntax.
        // Their independent type is checked at the host call boundary by
        // `UiCatalog::format_checked`; the AST gate checks names and leaves
        // the declared type authoritative.
        let _ = (expected_kind, actual_kind);
    }
}

pub(super) fn duplicate_client_specs(specs: &[ClientSpec]) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    specs
        .iter()
        .filter(|spec| !seen.insert(spec.client))
        .map(|spec| spec.client.key().to_owned())
        .collect()
}

pub(super) fn known_client(client: Client) -> bool {
    Client::ALL.contains(&client)
}
