//! Identity of the catalogue captured by an immutable trial run.
use crate::localisation::catalogue::Catalogue;
use std::sync::Arc;

enum CatalogueInput {
    SourceOnly,
    Localised(Option<Arc<()>>),
}

pub(crate) struct Snapshot {
    pub caption: String,
    catalogue: CatalogueInput,
}
impl Snapshot {
    pub fn capture(caption: String, localised: bool, catalogue: Option<&Catalogue>) -> Self {
        Self {
            caption,
            catalogue: if localised {
                CatalogueInput::Localised(catalogue.map(Catalogue::revision))
            } else {
                CatalogueInput::SourceOnly
            },
        }
    }
    pub fn stale(&self, catalogue: Option<&Catalogue>) -> bool {
        match (&self.catalogue, catalogue) {
            (CatalogueInput::SourceOnly, _) | (CatalogueInput::Localised(None), None) => false,
            (CatalogueInput::Localised(Some(before)), Some(now)) => {
                !Arc::ptr_eq(before, &now.revision())
            }
            _ => true,
        }
    }
}
