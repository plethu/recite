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

impl crate::Document {
    /// Caller-owned variables used by interpolation across the current project snapshot.
    pub fn preview_bindings(&self) -> Vec<recite_core::ast::InterpolationBinding> {
        let mut bindings = std::collections::BTreeMap::new();
        for document in self.kernel().snapshot().documents() {
            let parsed = recite_parser::parse(document.key().as_str(), document.source_text())
                .lower_source_file();
            parsed
                .source_file
                .visit_statements_depth_first(&mut |statement| {
                    let values = match statement {
                        recite_core::ast::Statement::Line(line) => &line.interpolation_bindings,
                        recite_core::ast::Statement::Choice(choice) => {
                            &choice.interpolation_bindings
                        }
                        _ => return,
                    };
                    for binding in values {
                        bindings
                            .entry(binding.value.clone())
                            .or_insert_with(|| binding.clone());
                    }
                });
        }
        bindings.into_values().collect()
    }
}
