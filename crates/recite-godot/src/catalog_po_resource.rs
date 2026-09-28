use godot::builtin::VarDictionary;
use godot::prelude::*;

use crate::adapter::{AdapterError, AdapterErrorKind};
use crate::catalog_resource::ReciteDialogueCatalogResource;

pub(crate) fn import_po_bytes(
    resource: &mut ReciteDialogueCatalogResource,
    locale: &str,
    source_name: &str,
    bytes: &[u8],
) -> Result<(), AdapterError> {
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => {
            return Err(AdapterError::with_detail(
                AdapterErrorKind::Localisation,
                format!("PO catalogue must be UTF-8: {error}"),
            ));
        }
    };
    resource.refresh_catalog()?;
    let result = resource.catalog.import_po(locale, source_name, text);
    if result.is_ok() {
        let mut record = VarDictionary::new();
        record.set("locale", locale);
        record.set("source_name", source_name);
        record.set("text", text);
        resource.serialized_po_sources.push(&record.to_variant());
    }
    result
}
