use godot::builtin::{GString, VarArray, VarDictionary, Variant};
use godot::prelude::*;

use crate::adapter::{AdapterError, AdapterErrorKind};
use crate::catalog::ReciteDialogueCatalog;
use crate::catalog_resource::ReciteDialogueCatalogResource;

pub(crate) fn decode_catalog(
    resource: &ReciteDialogueCatalogResource,
) -> Result<ReciteDialogueCatalog, AdapterError> {
    let mut catalog = ReciteDialogueCatalog::new();
    for (locale, header) in resource.serialized_plural_forms.iter_shared() {
        let locale = persisted_string(&locale, "plural form locale")?;
        let header = persisted_string(&header, "plural form header")?;
        catalog.set_plural_forms(&locale, header)?;
    }
    for value in resource.serialized_entries.iter_shared() {
        let record = persisted_dictionary(&value, "catalogue entry")?;
        let kind = persisted_field_string(&record, "kind")?;
        validate_record_keys(&record, &kind)?;
        let locale = persisted_field_string(&record, "locale")?;
        let id = persisted_field_string(&record, "id")?;
        let variant = optional_persisted_variant(&record, "variant")?;
        match kind.as_str() {
            "plural" => {
                let source_singular = persisted_field_string(&record, "source_singular")?;
                let source_plural = persisted_field_string(&record, "source_plural")?;
                let translations = persisted_field_array(&record, "translations")?;
                let mut arms = Vec::new();
                for arm in translations.iter_shared() {
                    arms.push(persisted_string(&arm, "plural translation arm")?);
                }
                catalog.insert_plural(
                    &locale,
                    &id,
                    &source_singular,
                    &source_plural,
                    arms,
                    variant.as_deref(),
                )?;
            }
            "singular" => {
                let source_text = persisted_field_string(&record, "source_text")?;
                let translation = persisted_field_string(&record, "translation")?;
                let domain = match persisted_field_i64(&record, "domain")? {
                    0 => recite_runtime::localisation::TextDomain::Line,
                    1 => recite_runtime::localisation::TextDomain::Choice,
                    2 => recite_runtime::localisation::TextDomain::AvailabilityReason,
                    3 => recite_runtime::localisation::TextDomain::PresentationLabel,
                    _ => {
                        return Err(AdapterError::with_detail(
                            AdapterErrorKind::Localisation,
                            "serialized catalogue contains an unknown text domain",
                        ));
                    }
                };
                catalog.insert_for_domain(
                    &locale,
                    domain,
                    &id,
                    &source_text,
                    translation,
                    variant.as_deref(),
                )?;
            }
            _ => {
                return Err(serialized_catalog_error(
                    "catalogue entry kind must be `singular` or `plural`",
                ));
            }
        }
    }
    for value in resource.serialized_po_sources.iter_shared() {
        let record = persisted_dictionary(&value, "PO source")?;
        let locale = persisted_field_string(&record, "locale")?;
        let source_name = persisted_field_string(&record, "source_name")?;
        let text = persisted_field_string(&record, "text")?;
        catalog.import_po(&locale, &source_name, &text)?;
    }
    Ok(catalog)
}

pub(crate) fn remember_singular(
    resource: &mut ReciteDialogueCatalogResource,
    locale: &GString,
    id: &GString,
    source_text: &GString,
    translation: &GString,
    domain: i64,
    variant: &GString,
) {
    let mut record = VarDictionary::new();
    record.set("kind", "singular");
    record.set("locale", locale.to_string());
    record.set("id", id.to_string());
    record.set("source_text", source_text.to_string());
    record.set("translation", translation.to_string());
    record.set("domain", domain);
    record.set("variant", variant.to_string());
    resource.serialized_entries.push(&record.to_variant());
}

fn serialized_catalog_error(message: impl Into<String>) -> AdapterError {
    AdapterError::with_detail(AdapterErrorKind::Localisation, message)
}

fn persisted_field(record: &VarDictionary, field: &str) -> Result<Variant, AdapterError> {
    record.get(field).ok_or_else(|| {
        serialized_catalog_error(format!("serialized catalogue is missing `{field}`"))
    })
}

fn persisted_string(value: &Variant, field: &str) -> Result<String, AdapterError> {
    value
        .try_to::<GString>()
        .map(|value| value.to_string())
        .map_err(|error| {
            serialized_catalog_error(format!(
                "serialized catalogue `{field}` must be a string: {error}"
            ))
        })
}

fn persisted_field_string(record: &VarDictionary, field: &str) -> Result<String, AdapterError> {
    let value = persisted_field(record, field)?;
    persisted_string(&value, field)
}

fn persisted_field_i64(record: &VarDictionary, field: &str) -> Result<i64, AdapterError> {
    let value = persisted_field(record, field)?;
    value.try_to::<i64>().map_err(|error| {
        serialized_catalog_error(format!(
            "serialized catalogue `{field}` must be an integer: {error}"
        ))
    })
}

fn persisted_field_array(record: &VarDictionary, field: &str) -> Result<VarArray, AdapterError> {
    let value = persisted_field(record, field)?;
    value.try_to::<VarArray>().map_err(|error| {
        serialized_catalog_error(format!(
            "serialized catalogue `{field}` must be an array: {error}"
        ))
    })
}

fn persisted_dictionary(value: &Variant, field: &str) -> Result<VarDictionary, AdapterError> {
    value.try_to::<VarDictionary>().map_err(|error| {
        serialized_catalog_error(format!(
            "serialized catalogue `{field}` must be a dictionary: {error}"
        ))
    })
}

fn validate_record_keys(record: &VarDictionary, kind: &str) -> Result<(), AdapterError> {
    let allowed = match kind {
        "plural" => [
            "kind",
            "locale",
            "id",
            "source_singular",
            "source_plural",
            "translations",
            "variant",
        ]
        .as_slice(),
        "singular" => [
            "kind",
            "locale",
            "id",
            "source_text",
            "translation",
            "domain",
            "variant",
        ]
        .as_slice(),
        _ => return Ok(()),
    };
    for (key, _) in record.iter_shared() {
        let key = persisted_string(&key, "catalogue entry key")?;
        if !allowed.iter().any(|candidate| *candidate == key) {
            return Err(serialized_catalog_error(format!(
                "serialized catalogue contains unknown entry key `{key}`"
            )));
        }
    }
    Ok(())
}

fn optional_persisted_variant(
    record: &VarDictionary,
    field: &str,
) -> Result<Option<String>, AdapterError> {
    let value = persisted_field(record, field)?;
    if value.is_nil() {
        return Ok(None);
    }
    let value = persisted_string(&value, field)?;
    Ok((!value.is_empty()).then_some(value))
}
