use godot::builtin::{GString, VarArray, VarDictionary, Variant};
use godot::classes::{IResource, Resource};
use godot::prelude::*;

use crate::adapter::{AdapterError, AdapterErrorKind};
use crate::binding_types::ReciteOperationResult;
use crate::catalog::ReciteDialogueCatalog;

#[cfg(test)]
mod tests;

/// Godot-owned dialogue catalogue. Entries are copied into the resource and
/// can be shared by any number of dialogue nodes.
#[derive(GodotClass)]
#[class(init, base=Resource)]
pub struct ReciteDialogueCatalogResource {
    base: Base<Resource>,
    catalog: ReciteDialogueCatalog,
    /// Serializable Godot properties. The validated Rust catalogue is rebuilt
    /// from these fields after a Resource is deserialized.
    #[var]
    serialized_entries: VarArray,
    #[var]
    serialized_plural_forms: VarDictionary,
}

#[godot_api]
impl IResource for ReciteDialogueCatalogResource {}

#[godot_api]
impl ReciteDialogueCatalogResource {
    #[func]
    fn add_translation(
        &mut self,
        locale: GString,
        id: GString,
        source_text: GString,
        translation: GString,
        variant: GString,
    ) -> Gd<ReciteOperationResult> {
        if let Err(error) = self.refresh_catalog() {
            return crate::bindings::catalog_result(Err(error));
        }
        let result = self.catalog.insert_for_domain(
            &locale.to_string(),
            recite_runtime::TextDomain::Line,
            &id.to_string(),
            &source_text.to_string(),
            translation.to_string(),
            crate::bindings::optional_string(variant.clone()).as_deref(),
        );
        if result.is_ok() {
            self.remember_singular(&locale, &id, &source_text, &translation, 0, &variant);
        }
        crate::bindings::catalog_result(result)
    }

    #[func]
    fn add_choice_translation(
        &mut self,
        locale: GString,
        id: GString,
        source_text: GString,
        translation: GString,
        variant: GString,
    ) -> Gd<ReciteOperationResult> {
        if let Err(error) = self.refresh_catalog() {
            return crate::bindings::catalog_result(Err(error));
        }
        let result = self.catalog.insert_for_domain(
            &locale.to_string(),
            recite_runtime::TextDomain::Choice,
            &id.to_string(),
            &source_text.to_string(),
            translation.to_string(),
            crate::bindings::optional_string(variant.clone()).as_deref(),
        );
        if result.is_ok() {
            self.remember_singular(&locale, &id, &source_text, &translation, 1, &variant);
        }
        crate::bindings::catalog_result(result)
    }

    #[func]
    fn add_availability_reason_translation(
        &mut self,
        locale: GString,
        id: GString,
        source_text: GString,
        translation: GString,
        variant: GString,
    ) -> Gd<ReciteOperationResult> {
        if let Err(error) = self.refresh_catalog() {
            return crate::bindings::catalog_result(Err(error));
        }
        let result = self.catalog.insert_for_domain(
            &locale.to_string(),
            recite_runtime::TextDomain::AvailabilityReason,
            &id.to_string(),
            &source_text.to_string(),
            translation.to_string(),
            crate::bindings::optional_string(variant.clone()).as_deref(),
        );
        if result.is_ok() {
            self.remember_singular(&locale, &id, &source_text, &translation, 2, &variant);
        }
        crate::bindings::catalog_result(result)
    }

    #[func]
    fn add_presentation_label_translation(
        &mut self,
        locale: GString,
        id: GString,
        source_text: GString,
        translation: GString,
        variant: GString,
    ) -> Gd<ReciteOperationResult> {
        if let Err(error) = self.refresh_catalog() {
            return crate::bindings::catalog_result(Err(error));
        }
        let result = self.catalog.insert_for_domain(
            &locale.to_string(),
            recite_runtime::TextDomain::PresentationLabel,
            &id.to_string(),
            &source_text.to_string(),
            translation.to_string(),
            crate::bindings::optional_string(variant.clone()).as_deref(),
        );
        if result.is_ok() {
            self.remember_singular(&locale, &id, &source_text, &translation, 3, &variant);
        }
        crate::bindings::catalog_result(result)
    }

    #[func]
    fn add_plural_translation(
        &mut self,
        locale: GString,
        id: GString,
        source_singular: GString,
        source_plural: GString,
        translations: VarArray,
        variant: GString,
    ) -> Gd<ReciteOperationResult> {
        if let Err(error) = self.refresh_catalog() {
            return crate::bindings::catalog_result(Err(error));
        }
        let mut arms = Vec::new();
        for value in translations.iter_shared() {
            let Ok(value) = value.try_to::<GString>() else {
                return crate::bindings::catalog_result(Err(AdapterError::with_detail(
                    AdapterErrorKind::Localisation,
                    "plural translation arms must be strings",
                )));
            };
            arms.push(value.to_string());
        }
        let result = self.catalog.insert_plural(
            &locale.to_string(),
            &id.to_string(),
            &source_singular.to_string(),
            &source_plural.to_string(),
            arms,
            crate::bindings::optional_string(variant.clone()).as_deref(),
        );
        if result.is_ok() {
            let mut record = VarDictionary::new();
            record.set("kind", "plural");
            record.set("locale", locale.to_string());
            record.set("id", id.to_string());
            record.set("source_singular", source_singular.to_string());
            record.set("source_plural", source_plural.to_string());
            record.set("translations", &translations.to_variant());
            record.set("variant", variant.to_string());
            self.serialized_entries.push(&record.to_variant());
        }
        crate::bindings::catalog_result(result)
    }

    #[func]
    fn set_plural_forms(&mut self, locale: GString, header: GString) -> Gd<ReciteOperationResult> {
        if let Err(error) = self.refresh_catalog() {
            return crate::bindings::catalog_result(Err(error));
        }
        let result = self
            .catalog
            .set_plural_forms(&locale.to_string(), header.to_string());
        if result.is_ok() {
            self.serialized_plural_forms
                .set(locale.to_string(), header.to_string());
        }
        crate::bindings::catalog_result(result)
    }

    pub(crate) fn cloned_catalog(&self) -> Result<ReciteDialogueCatalog, AdapterError> {
        self.decode_catalog()
    }

    fn refresh_catalog(&mut self) -> Result<(), AdapterError> {
        self.catalog = self.decode_catalog()?;
        Ok(())
    }

    fn decode_catalog(&self) -> Result<ReciteDialogueCatalog, AdapterError> {
        let mut catalog = ReciteDialogueCatalog::new();
        for (locale, header) in self.serialized_plural_forms.iter_shared() {
            let locale = persisted_string(&locale, "plural form locale")?;
            let header = persisted_string(&header, "plural form header")?;
            catalog.set_plural_forms(&locale, header)?;
        }
        for value in self.serialized_entries.iter_shared() {
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
                        0 => recite_runtime::TextDomain::Line,
                        1 => recite_runtime::TextDomain::Choice,
                        2 => recite_runtime::TextDomain::AvailabilityReason,
                        3 => recite_runtime::TextDomain::PresentationLabel,
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
        Ok(catalog)
    }

    fn remember_singular(
        &mut self,
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
        self.serialized_entries.push(&record.to_variant());
    }
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
