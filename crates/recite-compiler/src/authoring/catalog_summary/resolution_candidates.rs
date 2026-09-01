use recite_core::LocaleId;

use super::super::CatalogSummaryError;
use super::super::locale::{canonicalize, fallback_chain};
use super::policy::{CatalogResolutionPolicy, CatalogVariant};

/// A concrete locale/variant candidate in policy order.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub struct CatalogFallbackCandidate {
    locale: LocaleId,
    variant: CatalogVariant,
}

impl CatalogFallbackCandidate {
    #[must_use]
    pub const fn locale(&self) -> &LocaleId {
        &self.locale
    }

    #[must_use]
    pub const fn variant(&self) -> &CatalogVariant {
        &self.variant
    }
}

/// The policy and resulting ordered candidates shared by every expected entry.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct CatalogResolution {
    requested_locale: Option<LocaleId>,
    default_locale: Option<LocaleId>,
    candidates: Vec<CatalogFallbackCandidate>,
}

impl CatalogResolution {
    pub(crate) fn new(policy: &CatalogResolutionPolicy) -> Result<Self, CatalogSummaryError> {
        policy.validate()?;
        let requested_locale = policy
            .requested_locale
            .as_ref()
            .map(canonicalize)
            .transpose()?;
        let default_locale = policy
            .default_locale
            .as_ref()
            .map(canonicalize)
            .transpose()?;
        let fallback_locales = policy
            .fallback_locales
            .iter()
            .map(canonicalize)
            .collect::<Result<Vec<_>, _>>()?;

        let mut configured = Vec::new();
        if let Some(locale) = &requested_locale {
            configured.push(locale.clone());
        }
        if let Some(locale) = &default_locale {
            configured.push(locale.clone());
        }
        configured.extend(fallback_locales.iter().cloned());
        let mut seen = Vec::new();
        for locale in configured {
            if seen.contains(&locale) {
                return Err(CatalogSummaryError::FallbackCycle { locale });
            }
            seen.push(locale);
        }

        let mut locales = Vec::new();
        if let Some(locale) = &requested_locale {
            for candidate in fallback_chain(locale)? {
                push_unique(&mut locales, candidate);
            }
        }
        if let Some(locale) = &default_locale {
            push_unique(&mut locales, locale.clone());
        }
        for locale in fallback_locales {
            push_unique(&mut locales, locale);
        }
        let candidates = policy
            .variants
            .iter()
            .cloned()
            .flat_map(|variant| {
                locales
                    .iter()
                    .cloned()
                    .map(move |locale| CatalogFallbackCandidate {
                        locale,
                        variant: variant.clone(),
                    })
            })
            .collect::<Vec<_>>();
        Ok(Self {
            requested_locale,
            default_locale,
            candidates,
        })
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
    pub fn candidates(&self) -> &[CatalogFallbackCandidate] {
        &self.candidates
    }

    #[must_use]
    pub fn fallback_candidates(&self) -> &[CatalogFallbackCandidate] {
        self.candidates()
    }

    #[must_use]
    pub const fn is_source_only(&self) -> bool {
        self.requested_locale.is_none()
    }
}

fn push_unique(locales: &mut Vec<LocaleId>, locale: LocaleId) {
    if !locales.contains(&locale) {
        locales.push(locale);
    }
}
