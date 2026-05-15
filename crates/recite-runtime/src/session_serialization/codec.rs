use recite_core::CompiledDialogue;

use crate::session_snapshot::{DialogueSessionSnapshot, snapshot_session};
use crate::{DialogueError, DialogueSession};

use super::restore::restore_session;

pub fn encode_session_messagepack(session: &DialogueSession) -> Result<Vec<u8>, DialogueError> {
    rmp_serde::to_vec(&snapshot_session(session)).map_err(|error| {
        DialogueError::SessionSnapshotEncodeFailed {
            reason: error.to_string(),
        }
    })
}

/// Decodes MessagePack bytes and restores a session against the matching asset.
///
/// Restore validates asset identity, source fingerprints, statement ranges,
/// pending prompt shape, and compiled deferred-effect references. It does not
/// prove the bytes were produced by a previous traversal; authenticate save
/// data at the host layer when tamper resistance matters.
pub fn decode_session_messagepack(
    asset: &CompiledDialogue,
    bytes: &[u8],
) -> Result<DialogueSession, DialogueError> {
    let snapshot: DialogueSessionSnapshot = rmp_serde::from_slice(bytes).map_err(|error| {
        DialogueError::SessionSnapshotDecodeFailed {
            reason: error.to_string(),
        }
    })?;

    restore_session(asset, snapshot)
}
