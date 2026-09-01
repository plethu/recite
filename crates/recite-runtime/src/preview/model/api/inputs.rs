use recite_core::LocaleId;

use crate::{InterpolationValueProvider, LocaleProvider};

use super::ids::PreviewInputRevision;

#[non_exhaustive]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PreviewOptions {
    pub(crate) locale: Option<LocaleId>,
    pub(crate) variant: Option<String>,
}

impl PreviewOptions {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_locale(mut self, locale: LocaleId) -> Self {
        self.locale = Some(locale);
        self
    }

    #[must_use]
    pub fn with_variant(mut self, variant: impl Into<String>) -> Self {
        self.variant = Some(variant.into());
        self
    }

    #[must_use]
    pub fn locale(&self) -> Option<&LocaleId> {
        self.locale.as_ref()
    }

    #[must_use]
    pub fn variant(&self) -> Option<&str> {
        self.variant.as_deref()
    }

    pub(crate) fn with_optional_locale(mut self, locale: Option<LocaleId>) -> Self {
        self.locale = locale;
        self
    }

    pub(crate) fn with_optional_variant(mut self, variant: Option<String>) -> Self {
        self.variant = variant;
        self
    }
}

/// Inputs supplied by a preview host for one operation.
#[non_exhaustive]
#[derive(Clone, Copy, Default)]
pub struct PreviewInputs<'a> {
    pub(crate) locale_provider: Option<&'a dyn LocaleProvider>,
    pub(crate) interpolation_values: Option<&'a dyn InterpolationValueProvider>,
    pub(crate) revision: PreviewInputRevision,
}

impl<'a> PreviewInputs<'a> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_locale_provider(mut self, provider: &'a dyn LocaleProvider) -> Self {
        self.locale_provider = Some(provider);
        self
    }

    #[must_use]
    pub fn with_interpolation_values(mut self, values: &'a dyn InterpolationValueProvider) -> Self {
        self.interpolation_values = Some(values);
        self
    }

    /// Identifies the immutable host inputs used by a traversal attempt. A
    /// pending condition can only be resumed with the same revision. The
    /// revision is a caller-owned honesty boundary: the runtime compares the
    /// value, but cannot detect mutation behind a provider reference that is
    /// reused under the same revision.
    #[must_use]
    pub fn with_revision(mut self, revision: impl Into<PreviewInputRevision>) -> Self {
        self.revision = revision.into();
        self
    }

    #[must_use]
    pub fn revision(&self) -> PreviewInputRevision {
        self.revision
    }
}
