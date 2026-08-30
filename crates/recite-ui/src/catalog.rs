use std::{collections::BTreeMap, fmt};

use fluent_bundle::{FluentBundle, FluentResource};
use unic_langid::LanguageIdentifier;

use crate::{DEFAULT_LOCALE, DEFAULT_RESOURCE, MsgId, UiArgs};

mod diagnostics;
mod renderer;
use diagnostics::ResourceRegistry;

pub use renderer::{RenderedDiagnostic, RenderedRelatedDiagnostic};

pub struct UiCatalog {
    requested: LanguageIdentifier,
    bundles: BTreeMap<String, FluentBundle<FluentResource>>,
    resource_registry: ResourceRegistry,
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CatalogError {
    #[error("invalid locale `{locale}`: {source}")]
    Locale {
        locale: String,
        source: unic_langid::LanguageIdentifierError,
    },
    #[error("failed to parse {locale} Fluent resource: {details}")]
    Malformed { locale: String, details: String },
    #[error("{locale} UI resource is incomplete or cannot be resolved: {details}")]
    InvalidResource { locale: String, details: String },
    #[error("missing default Fluent catalog {DEFAULT_LOCALE}")]
    MissingDefault,
    #[error("default Fluent catalog is missing {0}")]
    MissingMessage(String),
    #[error("resource `{id}` is missing argument `{name}`")]
    MissingArgument { id: String, name: String },
    #[error("resource `{id}` has undeclared argument `{name}`")]
    ExtraArgument { id: String, name: String },
    #[error("resource `{id}` argument `{name}` has type {actual:?}, expected {expected:?}")]
    ArgumentTypeMismatch {
        id: String,
        name: String,
        expected: crate::UiArgType,
        actual: crate::UiArgType,
    },
    #[error("unsupported diagnostic argument value for resource `{id}` argument `{name}`")]
    UnsupportedDiagnosticArgument { id: String, name: String },
    #[error("failed to resolve `{id}`: {details}")]
    Resolution { id: String, details: String },
}

impl UiCatalog {
    pub fn load(locale: &crate::UiLocale) -> Result<Self, CatalogError> {
        let requested = locale.resolve();
        Self::from_resources(
            requested,
            [(langid(DEFAULT_LOCALE), DEFAULT_RESOURCE.to_owned())],
        )
    }

    pub fn from_resources(
        requested: LanguageIdentifier,
        resources: impl IntoIterator<Item = (LanguageIdentifier, String)>,
    ) -> Result<Self, CatalogError> {
        let contract = crate::UiContract::default();
        // Build the owned registry once at catalog construction. Formatting
        // performs lookups only; it never reconstructs the ~246-entry spec set.
        let resource_registry = ResourceRegistry::from_contract(&contract);
        let mut bundles = BTreeMap::new();
        for (locale, source) in resources {
            let key = locale.to_string();
            if let Err(error) = contract.validate(&source) {
                if key == DEFAULT_LOCALE {
                    return Err(CatalogError::InvalidResource {
                        locale: key,
                        details: error.to_string(),
                    });
                }
                // Non-default locales are fixtures until they are complete;
                // skip the entire bundle so fallback remains atomic.
                continue;
            }
            let resource = match FluentResource::try_new(source) {
                Ok(resource) => resource,
                Err((_, errors)) if key == DEFAULT_LOCALE => {
                    return Err(CatalogError::Malformed {
                        locale: key,
                        details: format!("{errors:?}"),
                    });
                }
                Err(_) => continue,
            };
            let mut bundle = FluentBundle::new(vec![locale]);
            bundle.set_use_isolating(false);
            if let Err(errors) = bundle.add_resource(resource) {
                if key == DEFAULT_LOCALE {
                    return Err(CatalogError::Malformed {
                        locale: key,
                        details: format!("{errors:?}"),
                    });
                }
                continue;
            }
            bundles.insert(key, bundle);
        }
        if !bundles.contains_key(DEFAULT_LOCALE) {
            return Err(CatalogError::MissingDefault);
        }
        for id in MsgId::ALL {
            if bundles
                .get(DEFAULT_LOCALE)
                .and_then(|bundle| bundle.get_message(id.key()))
                .is_none()
            {
                return Err(CatalogError::MissingMessage(id.key().to_owned()));
            }
        }
        Ok(Self {
            requested,
            bundles,
            resource_registry,
        })
    }

    pub fn format(&self, id: MsgId, args: &UiArgs) -> String {
        self.format_checked(id, args)
            .unwrap_or_else(|error| self.emergency_text(id.key(), error))
    }

    /// Render a deterministic, human-readable emergency value when a caller
    /// violates the checked argument contract. Returning a raw message ID
    /// would make a broken catalog look like successful UI output.
    fn emergency_text(&self, id: &str, error: CatalogError) -> String {
        format!("[UI text unavailable: {id} ({error})]")
    }

    pub fn format_checked(&self, id: MsgId, args: &UiArgs) -> Result<String, CatalogError> {
        let resource_id = id.resource_id();
        self.format_resource_checked(&resource_id, args)
    }

    pub fn text(&self, id: MsgId) -> String {
        self.format(id, &BTreeMap::new())
    }

    pub fn format_pairs<I, K, V>(&self, id: MsgId, args: I) -> String
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<crate::UiArg>,
    {
        let args = args
            .into_iter()
            .map(|(name, value)| (name.into(), value.into()))
            .collect();
        self.format(id, &args)
    }

    pub fn format_args(&self, id: MsgId, args: &UiArgs) -> String {
        self.format(id, args)
    }

    pub fn requested_locale(&self) -> &LanguageIdentifier {
        &self.requested
    }
}

fn langid(value: &str) -> LanguageIdentifier {
    value
        .parse()
        .unwrap_or_else(|_| panic!("embedded locale is valid: {value}"))
}

impl fmt::Debug for UiCatalog {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("UiCatalog")
            .field("requested", &self.requested)
            .field("locales", &self.bundles.keys().collect::<Vec<_>>())
            .field("resource_count", &self.resource_registry.len())
            .finish()
    }
}

#[cfg(test)]
mod tests;
