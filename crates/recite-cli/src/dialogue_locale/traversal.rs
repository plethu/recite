use recite_core::LocaleId;
use recite_runtime::LocaleProvider;

/// Loaded dialogue-localisation inputs passed to the shared preview driver.
///
/// This type deliberately contains no runtime session or traversal state. The runtime preview
/// owns those concerns; the CLI only supplies its explicit locale and provider.
#[derive(Clone, Copy)]
pub(crate) struct DialogueTraversalPreview<'a> {
    locale: &'a LocaleId,
    provider: &'a dyn LocaleProvider,
}

impl<'a> DialogueTraversalPreview<'a> {
    pub(crate) fn new(locale: &'a LocaleId, provider: &'a dyn LocaleProvider) -> Self {
        Self { locale, provider }
    }

    pub(crate) fn locale(&self) -> &'a LocaleId {
        self.locale
    }

    pub(crate) fn provider(&self) -> &'a dyn LocaleProvider {
        self.provider
    }
}
