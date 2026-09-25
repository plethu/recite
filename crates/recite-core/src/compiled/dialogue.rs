use std::ops::Deref;
use std::sync::OnceLock;

use super::{CompiledAssetEncodeError, CompiledDialoguePayload, ContentFingerprint};

/// Runtime-facing compiled dialogue asset.
///
/// Prepared assets expose their payload for reading. To change a compiled
/// program, consume it into a raw payload and prepare a new asset.
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

    #[must_use]
    pub fn into_payload(self) -> CompiledDialoguePayload {
        self.payload
    }

    /// Borrow the canonical full-payload identity, preparing it on first use.
    ///
    /// The identity remains valid for the lifetime of this immutable asset.
    pub fn content_fingerprint(&self) -> Result<&ContentFingerprint, &CompiledAssetEncodeError> {
        self.content_fingerprint
            .get_or_init(|| {
                super::fingerprint::compute_canonical_compiled_dialogue_fingerprint(self)
            })
            .as_ref()
    }

    pub(crate) fn prime_content_fingerprint(&self) -> Result<(), CompiledAssetEncodeError> {
        self.content_fingerprint().map(|_| ()).map_err(Clone::clone)
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
