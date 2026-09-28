use godot::builtin::{GString, PackedByteArray, VarArray, VarDictionary};
use godot::classes::{FileAccess, IResource, Resource};
use godot::prelude::*;

use crate::adapter::{AdapterError, AdapterErrorKind};
use crate::binding_types::ReciteOperationResult;
use crate::catalog::ReciteDialogueCatalog;

/// Godot-owned dialogue catalogue. Entries are copied into the resource and
/// can be shared by any number of dialogue nodes.
#[derive(GodotClass)]
#[class(init, base=Resource)]
pub struct ReciteDialogueCatalogResource {
    base: Base<Resource>,
    pub(crate) catalog: ReciteDialogueCatalog,
    /// Serializable Godot properties. The validated Rust catalogue is rebuilt
    /// from these fields after a Resource is deserialized.
    #[var(usage_flags = [STORAGE])]
    pub(crate) serialized_entries: VarArray,
    #[var(usage_flags = [STORAGE])]
    pub(crate) serialized_plural_forms: VarDictionary,
    #[var(usage_flags = [STORAGE])]
    pub(crate) serialized_po_sources: VarArray,
}

#[godot_api]
impl IResource for ReciteDialogueCatalogResource {}

#[godot_api]
impl ReciteDialogueCatalogResource {
    #[func]
    fn load_po_from_path(&mut self, locale: GString, path: GString) -> Gd<ReciteOperationResult> {
        let bytes = FileAccess::get_file_as_bytes(&path);
        if bytes.is_empty() {
            return crate::bindings::catalog_result(Err(AdapterError::with_detail(
                AdapterErrorKind::Localisation,
                format!("failed to read PO catalogue `{path}` through Godot FileAccess"),
            )));
        }
        self.load_po_from_bytes(locale, path, bytes)
    }

    #[func]
    fn load_po_from_bytes(
        &mut self,
        locale: GString,
        source_name: GString,
        bytes: PackedByteArray,
    ) -> Gd<ReciteOperationResult> {
        crate::bindings::catalog_result(crate::catalog_po_resource::import_po_bytes(
            self,
            &locale.to_string(),
            &source_name.to_string(),
            bytes.as_slice(),
        ))
    }

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
            recite_runtime::localisation::TextDomain::Line,
            &id.to_string(),
            &source_text.to_string(),
            translation.to_string(),
            crate::bindings::optional_string(variant.clone()).as_deref(),
        );
        if result.is_ok() {
            crate::catalog_persistence::remember_singular(
                self,
                &locale,
                &id,
                &source_text,
                &translation,
                0,
                &variant,
            );
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
            recite_runtime::localisation::TextDomain::Choice,
            &id.to_string(),
            &source_text.to_string(),
            translation.to_string(),
            crate::bindings::optional_string(variant.clone()).as_deref(),
        );
        if result.is_ok() {
            crate::catalog_persistence::remember_singular(
                self,
                &locale,
                &id,
                &source_text,
                &translation,
                1,
                &variant,
            );
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
            recite_runtime::localisation::TextDomain::AvailabilityReason,
            &id.to_string(),
            &source_text.to_string(),
            translation.to_string(),
            crate::bindings::optional_string(variant.clone()).as_deref(),
        );
        if result.is_ok() {
            crate::catalog_persistence::remember_singular(
                self,
                &locale,
                &id,
                &source_text,
                &translation,
                2,
                &variant,
            );
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
            recite_runtime::localisation::TextDomain::PresentationLabel,
            &id.to_string(),
            &source_text.to_string(),
            translation.to_string(),
            crate::bindings::optional_string(variant.clone()).as_deref(),
        );
        if result.is_ok() {
            crate::catalog_persistence::remember_singular(
                self,
                &locale,
                &id,
                &source_text,
                &translation,
                3,
                &variant,
            );
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
        crate::catalog_persistence::decode_catalog(self)
    }

    pub(crate) fn refresh_catalog(&mut self) -> Result<(), AdapterError> {
        self.catalog = crate::catalog_persistence::decode_catalog(self)?;
        Ok(())
    }
}
