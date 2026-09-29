use recite_core::{SourceAnchor, SourceId};

use crate::Provenance;

pub(crate) fn stable_id(file: &str, original: Option<&str>, provenance: &Provenance) -> String {
    if let Some(id) = original {
        if matches!(SourceId::parse(Some(id)), SourceId::Frozen { .. }) {
            return id.to_owned();
        }
        if SourceAnchor::new(id).is_ok() {
            return format!("imported_{id}@{id}");
        }
    }
    let mut hasher = blake3::Hasher::new();
    for value in [file, &provenance.record_key, original.unwrap_or("")] {
        hasher.update(&(value.len() as u64).to_le_bytes());
        hasher.update(value.as_bytes());
    }
    let anchor = &hasher.finalize().to_hex()[..20];
    format!("imported_{anchor}@{anchor}")
}

pub(crate) fn block_name(file: &str, name: &str) -> String {
    // Source identity keeps same-named blocks distinct across imported files.
    let mut hasher = blake3::Hasher::new();
    for value in [file, name] {
        hasher.update(&(value.len() as u64).to_le_bytes());
        hasher.update(value.as_bytes());
    }
    format!("block_{}", &hasher.finalize().to_hex()[..20])
}
