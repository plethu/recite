use std::collections::{BTreeMap, BTreeSet};

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use fluent_syntax::{ast::Entry, parser};

use super::ast::{check_arguments, collect_pattern, duplicate_client_specs, known_client};
use super::{ContractIssue, ResourceSpec, UiContract, UiContractError};
use crate::UiArgType;

impl UiContract {
    pub fn new(resources: Vec<ResourceSpec>, clients: Vec<super::ClientSpec>) -> Self {
        Self { resources, clients }
    }

    /// Parse and validate a Fluent resource against this inventory. The AST
    /// parser is used directly; no regex or source-text extraction is involved.
    pub fn validate(&self, source: &str) -> Result<(), UiContractError> {
        let mut issues = BTreeSet::new();
        let ast = match parser::parse(source) {
            Ok(ast) => ast,
            Err((ast, errors)) => {
                for error in errors {
                    issues.insert(ContractIssue::Malformed(error.to_string()));
                }
                ast
            }
        };

        let specs = self.index_resource_specs(&mut issues);
        let mut seen = BTreeSet::new();
        let mut observed = BTreeSet::new();
        for entry in ast.body {
            let Entry::Message(message) = entry else {
                if let Entry::Term(term) = entry {
                    issues.insert(ContractIssue::UnknownId(format!("-{}", term.id.name)));
                }
                continue;
            };
            self.validate_message(message, &specs, &mut seen, &mut observed, &mut issues);
        }
        self.validate_inventory(&specs, &observed, &mut issues);
        self.validate_clients(&mut issues);
        self.validate_resolution(source, &specs, &mut issues);
        if issues.is_empty() {
            Ok(())
        } else {
            Err(UiContractError {
                issues: issues.into_iter().collect(),
            })
        }
    }

    fn index_resource_specs<'a>(
        &'a self,
        issues: &mut BTreeSet<ContractIssue>,
    ) -> BTreeMap<&'a str, &'a ResourceSpec> {
        let mut specs = BTreeMap::new();
        for spec in &self.resources {
            if specs.insert(spec.id.as_str(), spec).is_some() {
                issues.insert(ContractIssue::DuplicateId(spec.id.to_string()));
            }
        }
        specs
    }

    fn validate_message(
        &self,
        message: fluent_syntax::ast::Message<&str>,
        specs: &BTreeMap<&str, &ResourceSpec>,
        seen: &mut BTreeSet<String>,
        observed: &mut BTreeSet<String>,
        issues: &mut BTreeSet<ContractIssue>,
    ) {
        let id = message.id.name;
        if !seen.insert(id.to_owned()) {
            issues.insert(ContractIssue::DuplicateId(id.to_owned()));
        }
        let Some(spec) = specs.get(id) else {
            issues.insert(ContractIssue::UnknownId(id.to_owned()));
            return;
        };
        observed.insert(id.to_owned());
        let mut variables = BTreeMap::new();
        if let Some(value) = &message.value {
            collect_pattern(value, &mut variables, id, issues);
        }
        let mut attributes = BTreeSet::new();
        for attribute in &message.attributes {
            let attribute_id = format!("{id}.{}", attribute.id.name);
            if !attributes.insert(attribute.id.name) {
                issues.insert(ContractIssue::DuplicateId(attribute_id.clone()));
            }
            if let Some(attribute_spec) = specs.get(attribute_id.as_str()) {
                observed.insert(attribute_id.clone());
                let mut attribute_vars = BTreeMap::new();
                collect_pattern(&attribute.value, &mut attribute_vars, &attribute_id, issues);
                check_arguments(attribute_spec, &attribute_id, &attribute_vars, issues);
            } else {
                issues.insert(ContractIssue::UnknownId(attribute_id));
            }
        }
        check_arguments(spec, id, &variables, issues);
    }

    fn validate_inventory(
        &self,
        specs: &BTreeMap<&str, &ResourceSpec>,
        observed: &BTreeSet<String>,
        issues: &mut BTreeSet<ContractIssue>,
    ) {
        for spec in specs.values() {
            if !observed.contains(spec.id.as_str()) {
                issues.insert(ContractIssue::MissingId(spec.id.to_string()));
            }
            for argument in &spec.duplicate_arguments {
                issues.insert(ContractIssue::DuplicateArgument {
                    id: spec.id.to_string(),
                    name: argument.clone(),
                });
            }
            for projection in &spec.projections {
                if projection.source != spec.id {
                    issues.insert(ContractIssue::ProjectionSourceMismatch {
                        id: spec.id.to_string(),
                        source: projection.source.to_string(),
                    });
                }
                if !spec.clients.contains(&projection.client) {
                    issues.insert(ContractIssue::UndeclaredProjection {
                        id: spec.id.to_string(),
                        client: projection.client.key().to_owned(),
                    });
                }
            }
        }
    }

    fn validate_clients(&self, issues: &mut BTreeSet<ContractIssue>) {
        for client in duplicate_client_specs(&self.clients) {
            issues.insert(ContractIssue::DuplicateClient(client));
        }
        for spec in &self.clients {
            if !known_client(spec.client) {
                issues.insert(ContractIssue::UnknownClient(spec.name.to_owned()));
            }
        }
    }

    #[expect(
        clippy::expect_used,
        reason = "this method owns the fixed en-US validation-locale invariant"
    )]
    fn validate_resolution(
        &self,
        source: &str,
        specs: &BTreeMap<&str, &ResourceSpec>,
        issues: &mut BTreeSet<ContractIssue>,
    ) {
        let Ok(resource) = FluentResource::try_new(source.to_owned()) else {
            return;
        };
        let locale = "en-US"
            .parse()
            .expect("embedded validation locale is valid");
        let mut bundle = FluentBundle::new(vec![locale]);
        if let Err(errors) = bundle.add_resource(resource) {
            issues.insert(ContractIssue::Resolution(format!(
                "resource conflicts: {errors:?}"
            )));
            return;
        }
        for (id, spec) in specs {
            let Some(message) = bundle.get_message(id) else {
                continue;
            };
            let mut args = FluentArgs::new();
            for (name, kind) in &spec.arguments {
                args.set(name, sample_value(*kind));
            }
            if let Some(pattern) = message.value() {
                check_pattern(&bundle, pattern, Some(&args), id, issues);
            }
            for attribute in message.attributes() {
                check_pattern(
                    &bundle,
                    attribute.value(),
                    Some(&args),
                    &format!("{id}.{}", attribute.id()),
                    issues,
                );
            }
        }
    }
}

fn sample_value(kind: UiArgType) -> FluentValue<'static> {
    match kind {
        UiArgType::String => FluentValue::String("sample".into()),
        UiArgType::Integer => FluentValue::Number(1.into()),
        UiArgType::Float => FluentValue::Number(1.5.into()),
        UiArgType::Boolean => FluentValue::String("true".into()),
    }
}

fn check_pattern(
    bundle: &FluentBundle<FluentResource>,
    pattern: &fluent_syntax::ast::Pattern<&str>,
    args: Option<&FluentArgs<'_>>,
    id: &str,
    issues: &mut BTreeSet<ContractIssue>,
) {
    let mut errors = Vec::new();
    let _ = bundle.format_pattern(pattern, args, &mut errors);
    if !errors.is_empty() {
        issues.insert(ContractIssue::Resolution(format!("{id}: {errors:?}")));
    }
}
