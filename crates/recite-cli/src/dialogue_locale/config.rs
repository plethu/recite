use std::collections::BTreeMap;
use std::path::PathBuf;

use recite_core::LocaleId;
use unic_langid::LanguageIdentifier;

use super::{
    DialogueCatalogProvider, DialogueCatalogSource, DialogueTraversalPreview,
    catalog::locale_fallbacks,
};
use crate::error::CliError;

#[derive(Clone, Debug)]
pub(crate) struct DialoguePreviewConfig {
    pub(crate) locale: LocaleId,
    pub(crate) catalogs: Vec<DialogueCatalogSource>,
}

impl DialoguePreviewConfig {
    pub(crate) fn from_fixture(
        locale: Option<&str>,
        catalogs: &BTreeMap<String, Vec<PathBuf>>,
    ) -> Result<Option<Self>, CliError> {
        if locale.is_none() && !catalogs.is_empty() {
            return Err(CliError::DialogueCatalogMissingLocale);
        }
        let Some(locale) = locale else {
            return Ok(None);
        };
        let locale = parse_locale(locale, "[dialogue].locale")?;
        let mut sources = Vec::new();
        for (catalog_locale, paths) in catalogs {
            let catalog_locale = parse_locale(catalog_locale, "[dialogue.catalogs] key")?;
            for path in paths {
                sources.push(DialogueCatalogSource {
                    locale: catalog_locale.clone(),
                    path: path.clone(),
                });
            }
        }
        Ok(Some(Self {
            locale,
            catalogs: sources,
        }))
    }
}

pub(crate) fn dialogue_preview_from_play_args(
    locale: Option<String>,
    catalogs: Vec<String>,
) -> Result<Option<DialoguePreviewConfig>, CliError> {
    if locale.is_none() && !catalogs.is_empty() {
        return Err(CliError::DialogueCatalogMissingLocale);
    }
    let Some(locale) = locale else {
        return Ok(None);
    };
    let locale = parse_locale(&locale, "--dialogue-locale")?;
    let catalogs = catalogs
        .into_iter()
        .map(|spec| {
            let (locale, path) = spec
                .split_once('=')
                .ok_or_else(|| CliError::DialogueCatalogSpecInvalid { spec: spec.clone() })?;
            if locale.is_empty() || path.is_empty() {
                return Err(CliError::DialogueCatalogSpecInvalid { spec });
            }
            Ok(DialogueCatalogSource {
                locale: parse_locale(locale, "--dialogue-catalog locale")?,
                path: PathBuf::from(path),
            })
        })
        .collect::<Result<Vec<_>, CliError>>()?;
    Ok(Some(DialoguePreviewConfig { locale, catalogs }))
}

pub(crate) struct LoadedDialoguePreview {
    locale: LocaleId,
    provider: DialogueCatalogProvider,
}

impl LoadedDialoguePreview {
    pub(crate) fn load(config: DialoguePreviewConfig) -> Result<Self, CliError> {
        Ok(Self {
            locale: config.locale,
            provider: DialogueCatalogProvider::load(config.catalogs)?,
        })
    }

    pub(crate) fn traversal_preview(&self) -> DialogueTraversalPreview<'_> {
        DialogueTraversalPreview::new(&self.locale, &self.provider)
    }

    pub(crate) fn locale_fallbacks(&self) -> Vec<String> {
        locale_fallbacks(self.locale.as_str())
    }
}

fn parse_locale(value: &str, field: &'static str) -> Result<LocaleId, CliError> {
    let locale =
        value
            .parse::<LanguageIdentifier>()
            .map_err(|_| CliError::DialogueLocaleInvalid {
                field,
                locale: value.to_owned(),
            })?;
    LocaleId::new(locale.to_string()).map_err(CliError::Core)
}
