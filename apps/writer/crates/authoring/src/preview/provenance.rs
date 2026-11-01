//! Transient input provenance travels with the runtime it describes.
use std::sync::Arc;

#[derive(Clone, Debug, Default)]
pub struct CatalogueRevision(Arc<()>);

impl PartialEq for CatalogueRevision {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for CatalogueRevision {}

#[derive(Clone, Debug)]
pub enum PreviewCatalogue {
    SourceOnly,
    Localised(Option<CatalogueRevision>),
}

#[derive(Clone, Debug)]
pub struct PreviewProvenance {
    pub caption: String,
    pub summary: String,
    catalogue: PreviewCatalogue,
}

impl PreviewProvenance {
    pub fn new(caption: String, catalogue: PreviewCatalogue) -> Self {
        Self {
            summary: caption.clone(),
            caption,
            catalogue,
        }
    }

    pub fn catalogue(&self) -> &PreviewCatalogue {
        &self.catalogue
    }
}
