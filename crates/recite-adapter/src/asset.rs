use std::sync::Arc;

use recite_core::compiled::{CompiledDialogue, decode_compiled_dialogue_messagepack};

use crate::{AdapterError, AdapterErrorKind, AdapterResult};

/// An immutable compiled asset that can be shared between adapter owners.
#[derive(Clone, Debug)]
pub struct LoadedDialogue(Arc<CompiledDialogue>);

impl LoadedDialogue {
    /// Validates the canonical payload identity once before making the asset live.
    pub fn new(dialogue: CompiledDialogue) -> AdapterResult<Self> {
        Self::from_shared(Arc::new(dialogue))
    }

    /// Decodes and validates an authored compiled asset.
    pub fn from_bytes(bytes: &[u8]) -> AdapterResult<Self> {
        Self::new(decode_compiled_dialogue_messagepack(bytes)?)
    }

    /// Shares a compiled asset without exposing mutable payload access.
    pub fn from_shared(dialogue: Arc<CompiledDialogue>) -> AdapterResult<Self> {
        dialogue.content_fingerprint().map_err(|error| {
            AdapterError::with_detail(AdapterErrorKind::AssetLoadOrDecode, error.to_string())
        })?;
        Ok(Self(dialogue))
    }

    #[must_use]
    pub fn dialogue(&self) -> &CompiledDialogue {
        &self.0
    }

    pub fn content_fingerprint(&self) -> AdapterResult<&recite_core::compiled::ContentFingerprint> {
        self.0.content_fingerprint().map_err(|error| {
            AdapterError::with_detail(AdapterErrorKind::AssetLoadOrDecode, error.to_string())
        })
    }

    /// A readable rendering of the validated canonical compiled-payload identity.
    pub fn content_identity(&self) -> AdapterResult<String> {
        use std::fmt::Write;

        let fingerprint = self.content_fingerprint()?;
        let mut identity = format!("{}:", fingerprint.algorithm().as_str());
        for byte in fingerprint.digest().as_bytes() {
            let _ = write!(&mut identity, "{byte:02x}");
        }
        Ok(identity)
    }

    #[must_use]
    pub fn asset_id(&self) -> &str {
        self.0.header.asset_id.as_str()
    }
}
