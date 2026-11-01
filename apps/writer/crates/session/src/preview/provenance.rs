//! Catalogue policy remains here; the immutable provenance belongs to its runtime.
use crate::localisation::catalogue::Catalogue;
use recite_writer_model::{PreviewCatalogue, PreviewProvenance};

pub fn capture_provenance(
    caption: String,
    localised: bool,
    catalogue: Option<&Catalogue>,
) -> PreviewProvenance {
    PreviewProvenance::new(
        caption,
        if localised {
            PreviewCatalogue::Localised(catalogue.map(Catalogue::revision))
        } else {
            PreviewCatalogue::SourceOnly
        },
    )
}

pub fn catalogue_stale(provenance: &PreviewProvenance, catalogue: Option<&Catalogue>) -> bool {
    match (provenance.catalogue(), catalogue) {
        (PreviewCatalogue::SourceOnly, _) | (PreviewCatalogue::Localised(None), None) => false,
        (PreviewCatalogue::Localised(Some(before)), Some(now)) => before != &now.revision(),
        _ => true,
    }
}
