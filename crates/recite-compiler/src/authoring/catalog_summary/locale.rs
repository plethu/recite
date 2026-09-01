use recite_core::{LocaleId, PoDocument};
use unic_langid::LanguageIdentifier;

use super::{CatalogIdentity, CatalogSummaryError};

pub(super) fn canonicalize(locale: &LocaleId) -> Result<LocaleId, CatalogSummaryError> {
    canonicalize_value(locale.as_str())
}

pub(super) fn canonicalize_value(value: &str) -> Result<LocaleId, CatalogSummaryError> {
    let parsed =
        value
            .parse::<LanguageIdentifier>()
            .map_err(|_| CatalogSummaryError::InvalidLocale {
                locale: value.to_owned(),
            })?;
    LocaleId::new(parsed.to_string()).map_err(|_| CatalogSummaryError::InvalidLocale {
        locale: value.to_owned(),
    })
}

pub(super) fn declared_language(
    document: &PoDocument,
    identity: &CatalogIdentity,
) -> Result<Option<LocaleId>, CatalogSummaryError> {
    let Some(header) = document
        .headers()
        .iter()
        .find(|header| header.key().eq_ignore_ascii_case("Language"))
    else {
        return Ok(None);
    };
    let language = canonicalize_value(header.value())?;
    if language != *identity.locale() {
        return Err(CatalogSummaryError::CatalogLocaleMismatch {
            identity: identity.clone(),
            language,
        });
    }
    Ok(Some(language))
}

pub(super) fn fallback_chain(locale: &LocaleId) -> Result<Vec<LocaleId>, CatalogSummaryError> {
    let canonical = canonicalize(locale)?;
    let parts = canonical.as_str().split('-').collect::<Vec<_>>();
    let mut chain = Vec::new();
    for length in (1..=parts.len()).rev() {
        let candidate = parts[..length].join("-");
        if let Ok(candidate) = canonicalize_value(&candidate)
            && !chain.contains(&candidate)
        {
            chain.push(candidate);
        }
    }
    Ok(chain)
}
