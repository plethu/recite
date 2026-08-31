use recite_core::LocaleId;

use super::CatalogSummaryError;

/// An explicit variant candidate. No variant is inferred from host state.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum CatalogVariant {
    Base,
    Named(String),
}

impl CatalogVariant {
    pub fn named(value: impl Into<String>) -> Result<Self, CatalogSummaryError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(CatalogSummaryError::EmptyVariant);
        }
        if value.contains('&') {
            return Err(CatalogSummaryError::InvalidVariant { variant: value });
        }
        Ok(Self::Named(value))
    }

    #[must_use]
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Base => None,
            Self::Named(value) => Some(value),
        }
    }
}

/// Caller-supplied locale and variant policy for authoring fallback.
///
/// The requested locale's BCP-47 parent chain is derived first (for example,
/// `pt-BR` then `pt`), followed by the explicitly configured default and
/// fallback locales. Variant selection remains explicit; no host state is
/// consulted.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct CatalogResolutionPolicy {
    pub(super) requested_locale: Option<LocaleId>,
    pub(super) default_locale: Option<LocaleId>,
    pub(super) fallback_locales: Vec<LocaleId>,
    pub(super) variants: Vec<CatalogVariant>,
}

impl CatalogResolutionPolicy {
    /// Construct a policy. `None` is an explicit source-only/unset locale.
    #[must_use]
    pub fn new(requested_locale: Option<LocaleId>) -> Self {
        Self {
            requested_locale,
            default_locale: None,
            fallback_locales: Vec::new(),
            variants: vec![CatalogVariant::Base],
        }
    }

    /// Construct the source-only policy explicitly.
    #[must_use]
    pub fn source_only() -> Self {
        Self::new(None)
    }

    #[must_use]
    pub const fn requested_locale(&self) -> Option<&LocaleId> {
        self.requested_locale.as_ref()
    }

    #[must_use]
    pub const fn default_locale(&self) -> Option<&LocaleId> {
        self.default_locale.as_ref()
    }

    #[must_use]
    pub fn fallback_locales(&self) -> &[LocaleId] {
        &self.fallback_locales
    }

    #[must_use]
    pub fn variants(&self) -> &[CatalogVariant] {
        &self.variants
    }

    #[must_use]
    pub fn with_default_locale(mut self, locale: LocaleId) -> Self {
        self.default_locale = Some(locale);
        self
    }

    #[must_use]
    pub fn with_fallback_locale(mut self, locale: LocaleId) -> Self {
        self.fallback_locales.push(locale);
        self
    }

    /// Replace the variant sequence after validating duplicate/empty values.
    pub fn with_variants(
        mut self,
        variants: impl IntoIterator<Item = CatalogVariant>,
    ) -> Result<Self, CatalogSummaryError> {
        let variants = variants.into_iter().collect::<Vec<_>>();
        if variants.is_empty() {
            return Err(CatalogSummaryError::EmptyVariantCandidates);
        }
        validate_variants(&variants)?;
        self.variants = variants;
        Ok(self)
    }

    /// Insert a named variant immediately before the `Base` candidate when present.
    pub fn with_variant(mut self, variant: impl Into<String>) -> Result<Self, CatalogSummaryError> {
        let variant = CatalogVariant::named(variant)?;
        if self.variants.contains(&variant) {
            return Err(CatalogSummaryError::DuplicateCandidate {
                candidate: format_variant(&variant),
            });
        }
        if let Some(base_index) = self
            .variants
            .iter()
            .position(|candidate| matches!(candidate, CatalogVariant::Base))
        {
            self.variants.insert(base_index, variant);
        } else {
            self.variants.push(variant);
        }
        Ok(self)
    }

    pub(super) fn validate(&self) -> Result<(), CatalogSummaryError> {
        if self.variants.is_empty() {
            return Err(CatalogSummaryError::EmptyVariantCandidates);
        }
        validate_variants(&self.variants)?;
        let mut locales = Vec::new();
        if let Some(locale) = &self.requested_locale {
            locales.push(locale.clone());
        }
        if let Some(locale) = &self.default_locale {
            locales.push(locale.clone());
        }
        locales.extend(self.fallback_locales.iter().cloned());
        let mut seen = Vec::new();
        for locale in locales {
            if seen.contains(&locale) {
                return Err(CatalogSummaryError::FallbackCycle { locale });
            }
            seen.push(locale);
        }
        if self.requested_locale.is_none()
            && (self.default_locale.is_some() || !self.fallback_locales.is_empty())
        {
            return Err(CatalogSummaryError::SourceOnlyHasLocaleCandidates);
        }
        Ok(())
    }
}

fn validate_variants(variants: &[CatalogVariant]) -> Result<(), CatalogSummaryError> {
    let mut seen = Vec::new();
    for variant in variants {
        if let CatalogVariant::Named(value) = variant {
            if value.trim().is_empty() {
                return Err(CatalogSummaryError::EmptyVariant);
            }
            if value.contains('&') {
                return Err(CatalogSummaryError::InvalidVariant {
                    variant: value.clone(),
                });
            }
        }
        if seen.contains(variant) {
            return Err(CatalogSummaryError::DuplicateCandidate {
                candidate: format_variant(variant),
            });
        }
        seen.push(variant.clone());
    }
    Ok(())
}

fn format_variant(variant: &CatalogVariant) -> String {
    variant
        .name()
        .map_or_else(|| "<base>".to_owned(), ToOwned::to_owned)
}
