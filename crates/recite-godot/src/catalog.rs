use std::collections::BTreeMap;

use recite_core::LocaleId;
use recite_runtime::TextDomain;

use crate::adapter_error::{AdapterError, AdapterErrorKind, AdapterResult};

mod provider;

/// Owned dialogue catalogue for the Godot adapter.
///
/// Entries are copied into the catalogue and resolved deterministically by
/// variant-context priority, then BCP-47 locale truncation. The runtime still
/// supplies source text when no entry matches. This type deliberately stores
/// dialogue content only; the session locale remains an explicit start option.
#[derive(Clone, Debug, Default)]
pub struct ReciteDialogueCatalog {
    translations: BTreeMap<CatalogKey, CatalogValue>,
    plural_forms: BTreeMap<String, String>,
}

impl ReciteDialogueCatalog {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a singular line or choice translation.
    pub fn insert(
        &mut self,
        locale: &str,
        id: &str,
        source_text: &str,
        translation: impl Into<String>,
    ) -> AdapterResult<()> {
        self.insert_for_domain(locale, TextDomain::Line, id, source_text, translation, None)
    }

    /// Adds a singular translation for an explicit dialogue text domain.
    pub fn insert_for_domain(
        &mut self,
        locale: &str,
        domain: TextDomain,
        id: &str,
        source_text: &str,
        translation: impl Into<String>,
        variant: Option<&str>,
    ) -> AdapterResult<()> {
        self.validate_entry(locale, id, source_text, variant)?;
        let translation = translation.into();
        validate_catalog_text(&translation, "translation")?;
        if !translation.is_empty() {
            recite_core::validate_translation_placeholders(source_text, &translation).map_err(
                |error| {
                    AdapterError::with_detail(
                        AdapterErrorKind::Localisation,
                        format!("invalid translation placeholders: {error:?}"),
                    )
                },
            )?;
        }
        let key = CatalogKey::singular(locale, domain, id, source_text, variant);
        self.insert_value(key, CatalogValue::singular(translation))
    }

    /// Adds a gettext plural entry. Empty arms intentionally remain missing
    /// translations and therefore exercise the normal source fallback.
    pub fn insert_plural(
        &mut self,
        locale: &str,
        id: &str,
        source_singular: &str,
        source_plural: &str,
        translations: Vec<String>,
        variant: Option<&str>,
    ) -> AdapterResult<()> {
        self.validate_entry(locale, id, source_singular, variant)?;
        if source_plural.is_empty() || translations.is_empty() {
            return Err(AdapterError::with_detail(
                AdapterErrorKind::Localisation,
                "plural entries require a source plural form and at least one arm",
            ));
        }
        let locale = valid_locale(locale)?;
        let Some(header) = self.plural_forms.get(locale.as_str()) else {
            return Err(AdapterError::with_detail(
                AdapterErrorKind::Localisation,
                format!("no plural rule installed for `{locale}`"),
            ));
        };
        let arm_count = recite_core::validate_plural_rule(header).map_err(|error| {
            AdapterError::with_detail(
                AdapterErrorKind::Localisation,
                format!("invalid plural rule for `{locale}`: {error:?}"),
            )
        })?;
        if translations.len() != arm_count {
            return Err(AdapterError::with_detail(
                AdapterErrorKind::Localisation,
                format!("plural entries for `{locale}` require exactly {arm_count} arms"),
            ));
        }
        validate_catalog_text(source_plural, "source plural text")?;
        for (index, translation) in translations.iter().enumerate() {
            validate_catalog_text(translation, "plural translation")?;
            if !translation.is_empty() {
                let source = if index == 0 {
                    source_singular
                } else {
                    source_plural
                };
                recite_core::validate_translation_placeholders(source, translation).map_err(
                    |error| {
                        AdapterError::with_detail(
                            AdapterErrorKind::Localisation,
                            format!("invalid plural translation placeholders: {error:?}"),
                        )
                    },
                )?;
            }
        }
        let key = CatalogKey::plural(
            locale.as_str(),
            TextDomain::Line,
            id,
            source_singular,
            source_plural,
            variant,
        );
        self.insert_value(key, CatalogValue { translations })
    }

    /// Installs and validates the locale's gettext `Plural-Forms` header.
    pub fn set_plural_forms(
        &mut self,
        locale: &str,
        header: impl Into<String>,
    ) -> AdapterResult<()> {
        let locale = valid_locale(locale)?;
        let header = header.into();
        validate_catalog_text(&header, "plural forms header")?;
        let arm_count = recite_core::validate_plural_rule(&header).map_err(|error| {
            AdapterError::with_detail(
                AdapterErrorKind::Localisation,
                format!("invalid plural rule for `{locale}`: {error}"),
            )
        })?;
        if self.translations.iter().any(|(key, value)| {
            key.locale == locale.as_str()
                && key.plural_source_text.is_some()
                && value.translations.len() != arm_count
        }) {
            return Err(AdapterError::with_detail(
                AdapterErrorKind::Localisation,
                format!("plural entries for `{locale}` do not match nplurals={arm_count}"),
            ));
        }
        self.plural_forms.insert(locale.as_str().to_owned(), header);
        Ok(())
    }

    fn validate_entry(
        &self,
        locale: &str,
        id: &str,
        source_text: &str,
        variant: Option<&str>,
    ) -> AdapterResult<()> {
        valid_locale(locale)?;
        if id.is_empty() || source_text.is_empty() {
            return Err(AdapterError::with_detail(
                AdapterErrorKind::Localisation,
                "catalogue IDs and source text must not be empty",
            ));
        }
        validate_catalog_text(id, "catalogue ID")?;
        validate_catalog_text(source_text, "source text")?;
        if variant.is_some_and(str::is_empty) {
            return Err(AdapterError::with_detail(
                AdapterErrorKind::Localisation,
                "catalogue variant must not be empty",
            ));
        }
        if let Some(variant) = variant {
            validate_catalog_text(variant, "catalogue variant")?;
        }
        Ok(())
    }

    fn insert_value(&mut self, key: CatalogKey, value: CatalogValue) -> AdapterResult<()> {
        if let Some(existing) = self.translations.get(&key)
            && existing != &value
        {
            return Err(AdapterError::with_detail(
                AdapterErrorKind::Localisation,
                format!("conflicting catalogue entry for `{}`", key.context),
            ));
        }
        self.translations.insert(key, value);
        Ok(())
    }

    fn plural_entry(
        &self,
        locale: &str,
        domain: TextDomain,
        context: &str,
        source_singular: &str,
        source_plural: &str,
    ) -> Option<&CatalogValue> {
        self.translations.get(&CatalogKey {
            locale: locale.to_owned(),
            domain,
            context: context.to_owned(),
            source_text: source_singular.to_owned(),
            plural_source_text: Some(source_plural.to_owned()),
        })
    }
}

fn validate_catalog_text(value: &str, label: &str) -> AdapterResult<()> {
    if value.contains('\0') {
        return Err(AdapterError::with_detail(
            AdapterErrorKind::Localisation,
            format!("{label} must not contain NUL"),
        ));
    }
    Ok(())
}

fn valid_locale(value: &str) -> AdapterResult<LocaleId> {
    LocaleId::new(value).map_err(|error| {
        AdapterError::with_detail(
            AdapterErrorKind::Localisation,
            format!("invalid locale: {error}"),
        )
    })
}

pub(super) fn gettext_context(id: &str, domain: TextDomain) -> String {
    match domain {
        TextDomain::Line | TextDomain::Choice => id.to_owned(),
        TextDomain::AvailabilityReason => format!("availability_reason:{id}"),
        TextDomain::PresentationLabel => format!("presentation_label:{id}"),
    }
}

pub(super) fn contexts(context: &str, variant: Option<&str>) -> Vec<String> {
    variant
        .map(|variant| format!("{context}&{variant}"))
        .into_iter()
        .chain(std::iter::once(context.to_owned()))
        .collect()
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct CatalogKey {
    locale: String,
    domain: TextDomain,
    context: String,
    source_text: String,
    plural_source_text: Option<String>,
}

impl CatalogKey {
    fn singular(
        locale: &str,
        domain: TextDomain,
        id: &str,
        source_text: &str,
        variant: Option<&str>,
    ) -> Self {
        let context = gettext_context(id, domain);
        Self {
            locale: locale.to_owned(),
            domain,
            context: variant.map_or(context.clone(), |variant| format!("{context}&{variant}")),
            source_text: source_text.to_owned(),
            plural_source_text: None,
        }
    }

    fn plural(
        locale: &str,
        domain: TextDomain,
        id: &str,
        source_singular: &str,
        source_plural: &str,
        variant: Option<&str>,
    ) -> Self {
        let context = gettext_context(id, domain);
        Self {
            locale: locale.to_owned(),
            domain,
            context: variant.map_or(context.clone(), |variant| format!("{context}&{variant}")),
            source_text: source_singular.to_owned(),
            plural_source_text: Some(source_plural.to_owned()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CatalogValue {
    translations: Vec<String>,
}

impl CatalogValue {
    fn singular(translation: String) -> Self {
        Self {
            translations: vec![translation],
        }
    }
}
