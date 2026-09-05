use recite_core::DiagnosticPresentation;
use recite_core::DiagnosticRecord;
use recite_ui::{RenderedDiagnostic, UiArg, UiArgs, UiCatalog};
use unic_langid::LanguageIdentifier;

use crate::error::CliError;

use super::locale::UiLocale;

/// Compatibility facade for the CLI's existing call sites. Resource parsing,
/// fallback, and typed inventory ownership live in `recite-ui`.
pub(crate) struct Messages {
    catalog: UiCatalog,
}

pub(crate) use recite_ui::MsgId;

impl Messages {
    pub(crate) fn load(locale: &UiLocale) -> Result<Self, CliError> {
        UiCatalog::load(locale)
            .map(|catalog| Self { catalog })
            .map_err(|source| CliError::UiCatalog {
                source: source.to_string(),
            })
    }

    #[allow(
        dead_code,
        reason = "shared CLI test helpers construct alternate UI catalogs"
    )]
    pub(crate) fn from_resources(
        requested: LanguageIdentifier,
        resources: impl IntoIterator<Item = (LanguageIdentifier, String)>,
    ) -> Result<Self, String> {
        UiCatalog::from_resources(requested, resources)
            .map(|catalog| Self { catalog })
            .map_err(|error| error.to_string())
    }

    pub(crate) fn text(&self, id: MsgId) -> String {
        self.catalog.text(id)
    }

    pub(crate) fn format_args(&self, id: MsgId, args: &UiArgs) -> String {
        self.catalog.format(id, args)
    }

    pub(crate) fn render_diagnostic(
        &self,
        record: &DiagnosticRecord,
    ) -> Result<RenderedDiagnostic, recite_ui::CatalogError> {
        self.catalog.render_diagnostic(record)
    }

    pub(crate) fn format_presentation(&self, presentation: &DiagnosticPresentation) -> String {
        self.catalog
            .format_presentation(presentation)
            .unwrap_or_else(|error| {
                format!("[UI text unavailable: {} ({error})]", presentation.id())
            })
    }

    pub(crate) fn format<I, K, V>(&self, id: MsgId, args: I) -> String
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<UiArg>,
    {
        let args = args
            .into_iter()
            .map(|(name, value)| (name.into(), value.into()))
            .collect::<UiArgs>();
        self.catalog.format(id, &args)
    }
}
