use std::collections::BTreeSet;

use recite_core::{StableId, StableIdGenerator};

use crate::Provenance;

/// Stable IDs assigned while importing one file.
///
/// Source-tool IDs are kept when they fit Recite's grammar after lowercasing
/// (or prefixing `_` to a leading digit, as in Yarn's hex `#line:` tags), even
/// when duplicated, so validation reports the source's own collision. Other
/// records get a generated ID seeded from their provenance, so re-importing the
/// same file reproduces the same IDs, and generation skips IDs already taken.
#[derive(Default)]
pub(crate) struct ImportedIds {
    occupied: BTreeSet<StableId>,
}

impl ImportedIds {
    pub(crate) fn assign(
        &mut self,
        file: &str,
        original: Option<&str>,
        provenance: &Provenance,
    ) -> StableId {
        if let Some(id) = original.and_then(normalised_original) {
            self.occupied.insert(id.clone());
            return id;
        }
        let mut ids = StableIdGenerator::seeded(record_seed(file, original, provenance));
        loop {
            let id = ids.next_id();
            if self.occupied.insert(id.clone()) {
                return id;
            }
        }
    }
}

fn normalised_original(original: &str) -> Option<StableId> {
    let lowered = original.to_ascii_lowercase();
    StableId::new(lowered.clone())
        .or_else(|_| StableId::new(format!("_{lowered}")))
        .ok()
}

fn record_seed(file: &str, original: Option<&str>, provenance: &Provenance) -> u64 {
    let mut hasher = blake3::Hasher::new();
    for value in [file, &provenance.record_key, original.unwrap_or("")] {
        hasher.update(&(value.len() as u64).to_le_bytes());
        hasher.update(value.as_bytes());
    }
    let digest = hasher.finalize();
    let mut seed = [0; 8];
    seed.copy_from_slice(&digest.as_bytes()[..8]);
    u64::from_le_bytes(seed)
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
