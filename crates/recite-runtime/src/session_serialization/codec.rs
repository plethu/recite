use recite_core::compiled::{CompiledDialogue, messagepack_array_len, messagepack_u16};
use serde::Deserialize;
use std::io::Cursor;

use crate::session_snapshot::{DialogueSessionSnapshot, SessionSnapshotRef};
use crate::{DialogueError, DialogueSession};

use super::restore::restore_session;

/// Encodes a session running against `asset` as MessagePack save bytes.
pub fn encode_session_messagepack(
    asset: &CompiledDialogue,
    session: &DialogueSession,
) -> Result<Vec<u8>, DialogueError> {
    rmp_serde::to_vec(&SessionSnapshotRef::new(asset, session)?).map_err(|error| {
        DialogueError::SessionSnapshotEncodeFailed {
            reason: error.to_string(),
        }
    })
}

/// Decodes MessagePack bytes and restores a session against the matching asset.
///
/// Restore validates asset identity, the canonical compiled payload fingerprint,
/// source fingerprints, statement ranges, pending prompt shape, pending
/// blocking-effect shape, and compiled deferred effect references. It does not
/// prove the bytes were produced by a previous traversal; authenticate save
/// data at the host layer when tamper resistance matters.
pub fn decode_session_messagepack(
    asset: &CompiledDialogue,
    bytes: &[u8],
) -> Result<DialogueSession, DialogueError> {
    restore_session(asset, decode_session_snapshot_messagepack(bytes)?)
}

/// Decodes MessagePack save bytes without an asset, for inspection or for
/// [`crate::resume::resume_session`]. It checks the encoding and snapshot
/// format version only.
pub fn decode_session_snapshot_messagepack(
    bytes: &[u8],
) -> Result<DialogueSessionSnapshot, DialogueError> {
    reject_unsupported_messagepack_snapshot_format(bytes)?;

    let mut cursor = Cursor::new(bytes);
    let mut deserializer = rmp_serde::Deserializer::new(&mut cursor);
    let snapshot = DialogueSessionSnapshot::deserialize(&mut deserializer).map_err(|error| {
        DialogueError::SessionSnapshotDecodeFailed {
            reason: error.to_string(),
        }
    })?;
    let consumed = cursor.position() as usize;
    if consumed != bytes.len() {
        return Err(DialogueError::SessionSnapshotDecodeFailed {
            reason: format!(
                "MessagePack snapshot has {} trailing bytes",
                bytes.len() - consumed
            ),
        });
    }

    Ok(snapshot)
}

fn reject_unsupported_messagepack_snapshot_format(bytes: &[u8]) -> Result<(), DialogueError> {
    let Some(snapshot_format_version) = compact_array_snapshot_format_version(bytes) else {
        return Ok(());
    };

    if snapshot_format_version == crate::snapshot::CURRENT_SESSION_SNAPSHOT_FORMAT_VERSION {
        Ok(())
    } else {
        Err(DialogueError::UnsupportedSessionSnapshotFormat {
            snapshot_format_version,
        })
    }
}

fn compact_array_snapshot_format_version(bytes: &[u8]) -> Option<u16> {
    let (item_count, mut offset) = messagepack_array_len(bytes)?;
    if item_count == 0 {
        return None;
    }

    messagepack_u16(bytes, &mut offset)
}
