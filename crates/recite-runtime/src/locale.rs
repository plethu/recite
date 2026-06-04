use recite_core::LocaleId;

/// Localisable text category supplied to runtime locale providers.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TextDomain {
    Line,
    Choice,
    AvailabilityReason,
}

/// Runtime translation lookup surface.
///
/// Recite passes both the stable localisable ID and source text to support
/// gettext-style providers. When `variant` is present, providers should try the
/// variant-specific lookup key before their non-variant key, then return
/// `None` when no translation exists so the runtime can fall back to
/// `source_text`.
pub trait LocaleProvider {
    fn lookup(
        &self,
        id: &str,
        source_text: &str,
        domain: TextDomain,
        locale: &LocaleId,
        variant: Option<&str>,
    ) -> Option<String>;
}
