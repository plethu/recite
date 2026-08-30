use recite_core::{
    CompiledAssetEncodeError, CompiledAssetId, ContentFingerprint, FingerprintAlgorithm,
    FingerprintDigest,
};

use crate::{DialogueContentFingerprintSnapshot, PreviewError};

/// The canonical build identity used to distinguish compiled payload revisions.
///
/// The payload fingerprint is the BLAKE3 digest of the exact core-owned v0
/// MessagePack payload. It is not authenticated: persisted preview snapshots
/// remain a host trust/corruption boundary, and this type does not duplicate
/// the encoder's semantic authority.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewAssetRevision {
    asset_id: CompiledAssetId,
    payload_fingerprint: ContentFingerprint,
}

impl PreviewAssetRevision {
    pub(crate) fn from_asset(
        asset: &recite_core::CompiledDialogue,
    ) -> Result<Self, CompiledAssetEncodeError> {
        Ok(Self {
            asset_id: asset.header.asset_id.clone(),
            payload_fingerprint: recite_core::canonical_compiled_dialogue_fingerprint(asset)?,
        })
    }

    pub(crate) fn from_parts(
        asset_id: CompiledAssetId,
        payload_fingerprint: ContentFingerprint,
    ) -> Self {
        Self {
            asset_id,
            payload_fingerprint,
        }
    }

    #[must_use]
    pub fn asset_id(&self) -> &CompiledAssetId {
        &self.asset_id
    }

    #[must_use]
    pub fn payload_fingerprint(&self) -> &ContentFingerprint {
        &self.payload_fingerprint
    }

    pub(crate) fn fingerprint_snapshot(&self) -> DialogueContentFingerprintSnapshot {
        DialogueContentFingerprintSnapshot {
            algorithm: self.payload_fingerprint.algorithm().as_str().to_owned(),
            digest: self.payload_fingerprint.digest().as_bytes().to_vec(),
        }
    }

    pub(crate) fn from_fingerprint_snapshot(
        asset_id: CompiledAssetId,
        snapshot: DialogueContentFingerprintSnapshot,
    ) -> Result<Self, PreviewError> {
        let algorithm = FingerprintAlgorithm::new(snapshot.algorithm).map_err(invalid)?;
        let digest = FingerprintDigest::new(snapshot.digest).map_err(invalid)?;
        let fingerprint = ContentFingerprint::new(algorithm, digest).map_err(invalid)?;
        Ok(Self::from_parts(asset_id, fingerprint))
    }
}

fn invalid(error: impl std::fmt::Display) -> PreviewError {
    PreviewError::SnapshotDecodeFailed {
        reason: error.to_string(),
    }
}
