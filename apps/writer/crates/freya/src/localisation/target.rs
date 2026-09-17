//! Catalogue identity and the language used to obtain gettext plural metadata.
use super::messages::{MsgId, text as wording};
use unic_langid::LanguageIdentifier;

pub(super) const COFI: &str = "cy-x-cofi";

#[derive(Clone)]
pub(super) struct TargetLocale {
    tag: String,
    base: LanguageIdentifier,
}

impl TargetLocale {
    pub fn parse(value: &str) -> Result<Self, String> {
        let value = value.trim();
        let cofi = value.eq_ignore_ascii_case(COFI);
        let base_tag = if cofi { "cy" } else { value };
        let base: LanguageIdentifier = base_tag
            .parse()
            .map_err(|_| wording(MsgId::WriterInvalidLanguage))?;
        let registered = base_tag
            .parse::<language_tags::LanguageTag>()
            .is_ok_and(|tag| tag.validate().is_ok());
        let known = isolang::Language::from_639_1(base.language.as_str())
            .or_else(|| isolang::Language::from_639_3(base.language.as_str()));
        if !registered || known.is_none() || base.language.is_empty() {
            return Err(wording(MsgId::WriterInvalidLanguage));
        }
        Ok(Self {
            tag: if cofi { COFI.into() } else { base.to_string() },
            base,
        })
    }

    pub fn base(&self) -> &LanguageIdentifier {
        &self.base
    }
    pub fn is_cofi(&self) -> bool {
        self.tag == COFI
    }
}

impl std::fmt::Display for TargetLocale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.tag.fmt(f)
    }
}
