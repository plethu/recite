//! Explicit host inputs are consumed at restart and remain fixed for the trial.
use recite_compiler::authoring::{CatalogInput, CatalogResolutionPolicy};
use recite_runtime::{localisation::InterpolationValues, preview::PreviewOptions};

#[derive(Clone)]
pub struct PreviewSetup {
    pub policy: CatalogResolutionPolicy,
    pub catalogues: Vec<CatalogInput>,
    pub values: InterpolationValues,
}
impl Default for PreviewSetup {
    fn default() -> Self {
        Self {
            policy: CatalogResolutionPolicy::source_only(),
            catalogues: Vec::new(),
            values: InterpolationValues::new(),
        }
    }
}
impl PreviewSetup {
    pub(super) fn options(&self) -> PreviewOptions {
        let mut options = PreviewOptions::new();
        if let Some(locale) = self.policy.requested_locale() {
            options = options.with_locale(locale.clone());
        }
        if let Some(variant) = self.policy.variants().first().and_then(|v| v.name()) {
            options = options.with_variant(variant);
        }
        options
    }
}
