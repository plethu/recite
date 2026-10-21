//! Typed interpolation values and captured catalogue policy for a runtime trial.
use crate::localisation::catalogue::PreviewInput;
use crate::messages::{MsgId, text};
use recite_compiler::authoring::{CatalogInput, CatalogResolutionPolicy};
use recite_core::{LocaleId, ScalarValue, ast::InterpolationType};
use recite_writer_model::PreviewSetup;
use std::collections::BTreeMap;
#[derive(Default)]
pub struct Inputs {
    pub locale: String,
    pub variant: String,
    pub include_drafts: bool,
    pub values: Option<BTreeMap<String, String>>,
}
impl Inputs {
    pub(super) fn prepare(
        &self,
        source: &recite_writer_model::PreviewRequest,
        catalogue: Option<&PreviewInput>,
    ) -> Result<PreviewSetup, String> {
        let Self {
            locale,
            variant,
            values,
            ..
        } = self;
        let requested = if locale.is_empty() {
            None
        } else {
            Some(LocaleId::new(locale).map_err(|e| e.to_string())?)
        };
        let mut policy = CatalogResolutionPolicy::new(requested);
        if !variant.trim().is_empty() {
            policy = policy
                .with_variant(variant.trim())
                .map_err(|e| e.to_string())?;
        }
        let mut setup = PreviewSetup {
            policy,
            ..PreviewSetup::default()
        };
        if let Some(catalogue) = catalogue {
            let document = catalogue.prepare()?;
            let language = document
                .headers()
                .iter()
                .find(|h| h.key().eq_ignore_ascii_case("Language"))
                .ok_or_else(|| text(MsgId::WriterInvalidLanguage))?;
            let language =
                LocaleId::new(language.value().replace('_', "-")).map_err(|e| e.to_string())?;
            setup
                .catalogues
                .push(CatalogInput::from_document(language, document).map_err(|e| e.to_string())?);
        }
        let Some(values) = values else {
            return Ok(setup);
        };
        for binding in source.preview_bindings().iter() {
            let name = binding.value.trim_start_matches('$');
            let value = values.get(name).map_or(
                if binding.value_type == InterpolationType::Boolean {
                    "false"
                } else {
                    ""
                },
                String::as_str,
            );
            let value = parse_value(binding.value_type, value)
                .map_err(|error| format!("{name}: {error}"))?;
            setup.values.insert(name.into(), value);
        }
        Ok(setup)
    }
}

/// The same declared scalar validation serves worker capture and staged input controls.
pub fn parse_value(kind: InterpolationType, value: &str) -> Result<ScalarValue, String> {
    let invalid = || text(MsgId::WriterTrialInvalidValue);
    Ok(match kind {
        InterpolationType::String => ScalarValue::String(value.into()),
        InterpolationType::Integer => ScalarValue::Integer(value.parse().map_err(|_| invalid())?),
        InterpolationType::Float => {
            let value: f64 = value.parse().map_err(|_| invalid())?;
            if !value.is_finite() {
                return Err(invalid());
            }
            ScalarValue::Float(value)
        }
        InterpolationType::Boolean => ScalarValue::Boolean(value.parse().map_err(|_| invalid())?),
    })
}
