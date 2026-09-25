use crate::{
    editing::Writer,
    messages::{MsgId, text},
};
use recite_compiler::authoring::{CatalogInput, CatalogResolutionPolicy};
use recite_core::{LocaleId, ScalarValue, ast::InterpolationType};
use recite_writer_model::PreviewSetup;
use std::collections::BTreeMap;
pub(super) fn prepare(
    writer: Writer,
    locale: &str,
    variant: &str,
    include_drafts: bool,
    values: &BTreeMap<String, String>,
) -> Result<PreviewSetup, String> {
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
    if !locale.is_empty()
        && let Some(catalogue) = &writer.localisation.peek().catalogue
    {
        let document = catalogue.preview_document(include_drafts)?;
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
    let model = writer.buffers.model.peek();
    let model = model.as_ref().map_err(|e| e.to_string())?;
    for binding in model.document().preview_bindings() {
        let name = binding.value.trim_start_matches('$');
        let value = values.get(name).map_or(
            if binding.value_type == InterpolationType::Boolean {
                "false"
            } else {
                ""
            },
            String::as_str,
        );
        let invalid = || format!("{}: {}", name, text(MsgId::WriterTrialInvalidValue));
        let value = match binding.value_type {
            InterpolationType::String => ScalarValue::String(value.into()),
            InterpolationType::Integer => {
                ScalarValue::Integer(value.parse().map_err(|_| invalid())?)
            }
            InterpolationType::Float => {
                let value: f64 = value.parse().map_err(|_| invalid())?;
                if !value.is_finite() {
                    return Err(invalid());
                }
                ScalarValue::Float(value)
            }
            InterpolationType::Boolean => {
                ScalarValue::Boolean(value.parse().map_err(|_| invalid())?)
            }
        };
        setup.values.insert(name.into(), value);
    }
    Ok(setup)
}
