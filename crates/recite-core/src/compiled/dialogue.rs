use std::ops::{Deref, DerefMut};
use std::sync::OnceLock;

use super::{CompiledAssetEncodeError, CompiledDialoguePayload, ContentFingerprint};

/// Runtime-facing compiled dialogue asset.
///
/// The payload is kept separate from the identity cache so raw-model editing
/// remains available through `DerefMut`, while an exclusive borrow can always
/// invalidate the cached identity before exposing mutable data.
#[derive(Debug)]
pub struct CompiledDialogue {
    payload: CompiledDialoguePayload,
    content_fingerprint: OnceLock<Result<ContentFingerprint, CompiledAssetEncodeError>>,
}

impl CompiledDialogue {
    #[must_use]
    pub fn new(payload: CompiledDialoguePayload) -> Self {
        Self {
            payload,
            content_fingerprint: OnceLock::new(),
        }
    }

    /// Construct an asset and prepare its canonical content identity.
    pub fn prepare(payload: CompiledDialoguePayload) -> Result<Self, CompiledAssetEncodeError> {
        let dialogue = Self::new(payload);
        dialogue.prime_content_fingerprint()?;
        Ok(dialogue)
    }

    pub(crate) fn cached_content_fingerprint(
        &self,
    ) -> Result<&ContentFingerprint, &CompiledAssetEncodeError> {
        self.content_fingerprint
            .get_or_init(|| {
                super::fingerprint::compute_canonical_compiled_dialogue_fingerprint(self)
            })
            .as_ref()
    }

    pub(crate) fn prime_content_fingerprint(&self) -> Result<(), CompiledAssetEncodeError> {
        self.cached_content_fingerprint()
            .map(|_| ())
            .map_err(Clone::clone)
    }

    pub(crate) fn cache_content_fingerprint(
        &self,
        result: Result<ContentFingerprint, CompiledAssetEncodeError>,
    ) {
        let _ = self.content_fingerprint.set(result);
    }

    pub(crate) fn cache_canonical_bytes(&self, bytes: &[u8]) {
        self.cache_content_fingerprint(Ok(super::fingerprint::canonical_blake3_fingerprint(bytes)));
    }
}

impl Clone for CompiledDialogue {
    fn clone(&self) -> Self {
        let cloned = Self::new(self.payload.clone());
        if let Some(fingerprint) = self.content_fingerprint.get() {
            let _ = cloned.content_fingerprint.set(fingerprint.clone());
        }
        cloned
    }
}

impl PartialEq for CompiledDialogue {
    fn eq(&self, other: &Self) -> bool {
        self.payload == other.payload
    }
}

impl Deref for CompiledDialogue {
    type Target = CompiledDialoguePayload;

    fn deref(&self) -> &Self::Target {
        &self.payload
    }
}

impl DerefMut for CompiledDialogue {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let _ = self.content_fingerprint.take();
        &mut self.payload
    }
}
