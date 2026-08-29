use std::collections::BTreeMap;

use super::{CatalogError, UiCatalog};
use crate::{ResourceId, ResourceSpec, UiArgs, UiContract};

pub(super) struct ResourceRegistry {
    specs: BTreeMap<ResourceId, ResourceSpec>,
}

impl ResourceRegistry {
    pub(super) fn from_contract(contract: &UiContract) -> Self {
        Self {
            specs: contract
                .resources
                .iter()
                .cloned()
                .map(|spec| (spec.id.clone(), spec))
                .collect(),
        }
    }

    pub(super) fn get(&self, id: &ResourceId) -> Option<&ResourceSpec> {
        self.specs.get(id)
    }

    pub(super) fn len(&self) -> usize {
        self.specs.len()
    }
}

impl UiCatalog {
    /// Resolve the sole compatibility adapter for an unmigrated producer's
    /// deterministic English message. This is deliberately separate from
    /// localised diagnostic presentation resources.
    pub fn format_legacy_diagnostic_message(&self, message: &str) -> Result<String, CatalogError> {
        let id = ResourceId::new(crate::LEGACY_DIAGNOSTIC_RESOURCE)
            .map_err(|error| CatalogError::MissingMessage(error.to_string()))?;
        let args = [(
            "message".to_owned(),
            crate::UiArg::String(message.to_owned()),
        )]
        .into_iter()
        .collect();
        self.format_resource_checked(&id, &args)
    }

    /// Resolve a dynamic resource by its stable string identity.
    ///
    /// Static UI copy should continue to use [`crate::MsgId`]. Dynamic
    /// diagnostics use this path so their structured presentation IDs do not
    /// need to be added to the static Rust enum.
    pub fn format_resource(&self, id: &ResourceId, args: &UiArgs) -> String {
        self.format_resource_checked(id, args)
            .unwrap_or_else(|error| self.emergency_text(id.as_str(), error))
    }

    /// Resolve a dynamic resource while retaining typed missing/extra/type
    /// failures for callers that need an explicit contract result.
    pub fn format_resource_checked(
        &self,
        id: &ResourceId,
        args: &UiArgs,
    ) -> Result<String, CatalogError> {
        let spec = self
            .resource_registry
            .get(id)
            .ok_or_else(|| CatalogError::MissingMessage(id.to_string()))?;
        for name in spec.arguments.keys() {
            if !args.contains_key(name) {
                return Err(CatalogError::MissingArgument {
                    id: id.to_string(),
                    name: name.clone(),
                });
            }
        }
        for (name, value) in args {
            let Some(expected) = spec.arguments.get(name) else {
                return Err(CatalogError::ExtraArgument {
                    id: id.to_string(),
                    name: name.clone(),
                });
            };
            if *expected != value.kind() {
                return Err(CatalogError::ArgumentTypeMismatch {
                    id: id.to_string(),
                    name: name.clone(),
                    expected: *expected,
                    actual: value.kind(),
                });
            }
        }
        for locale in crate::fallback_chain(&self.requested) {
            let Some(bundle) = self.bundles.get(&locale.to_string()) else {
                continue;
            };
            let Some(message) = bundle.get_message(id.as_str()) else {
                continue;
            };
            let Some(pattern) = message.value() else {
                continue;
            };
            let fluent_args = crate::args::fluent_args(args);
            let mut errors = Vec::new();
            let formatted = bundle.format_pattern(pattern, Some(&fluent_args), &mut errors);
            if errors.is_empty() {
                return Ok(formatted.into_owned());
            }
            return Err(CatalogError::Resolution {
                id: id.to_string(),
                details: format!("{errors:?}"),
            });
        }
        Err(CatalogError::MissingMessage(id.to_string()))
    }

    /// Resolve a core structured diagnostic presentation through this shared
    /// Fluent boundary. Core remains independent of Fluent.
    pub fn format_presentation(
        &self,
        presentation: &recite_core::DiagnosticPresentation,
    ) -> Result<String, CatalogError> {
        let mut args = UiArgs::new();
        for (name, value) in presentation.arguments() {
            let value = match value {
                recite_core::DiagnosticArgumentValue::String(value) => {
                    crate::UiArg::String(value.clone())
                }
                recite_core::DiagnosticArgumentValue::Integer(value) => {
                    crate::UiArg::Integer(*value)
                }
                recite_core::DiagnosticArgumentValue::Float(value) => {
                    crate::UiArg::Float(value.as_f64())
                }
                recite_core::DiagnosticArgumentValue::Boolean(value) => {
                    crate::UiArg::Boolean(*value)
                }
                _ => {
                    return Err(CatalogError::UnsupportedDiagnosticArgument {
                        id: presentation.id().to_string(),
                        name: name.clone(),
                    });
                }
            };
            args.insert(name.clone(), value);
        }
        let id = ResourceId::new(presentation.id().as_str())
            .map_err(|error| CatalogError::MissingMessage(error.to_string()))?;
        self.format_resource_checked(&id, &args)
    }
}
