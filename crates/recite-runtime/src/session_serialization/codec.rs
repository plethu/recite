use recite_core::CompiledDialogue;
use serde::Deserialize;
use std::io::Cursor;

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
/// pending prompt shape, pending blocking-effect shape, and compiled deferred
/// effect references. It does not prove the bytes were produced by a previous
/// traversal; authenticate save data at the host layer when tamper resistance
/// matters.
pub fn decode_session_messagepack(
    asset: &CompiledDialogue,
    bytes: &[u8],
) -> Result<DialogueSession, DialogueError> {
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

    restore_session(asset, snapshot)
}

fn reject_unsupported_messagepack_snapshot_format(bytes: &[u8]) -> Result<(), DialogueError> {
    let Some(snapshot_format_version) = compact_array_snapshot_format_version(bytes) else {
        return Ok(());
    };

    if snapshot_format_version == crate::CURRENT_SESSION_SNAPSHOT_FORMAT_VERSION {
        Ok(())
    } else {
        Err(DialogueError::UnsupportedSessionSnapshotFormat {
            snapshot_format_version,
        })
    }
}

fn compact_array_snapshot_format_version(bytes: &[u8]) -> Option<u16> {
    let (item_count, mut offset) = read_array_len(bytes)?;
    if item_count == 0 {
        return None;
    }

    read_u16(bytes, &mut offset)
}

fn read_array_len(bytes: &[u8]) -> Option<(u32, usize)> {
    let marker = *bytes.first()?;
    match marker {
        0x90..=0x9f => Some((u32::from(marker & 0x0f), 1)),
        0xdc => Some((u32::from(u16::from_be_bytes(read_array(bytes, 1)?)), 3)),
        0xdd => Some((u32::from_be_bytes(read_array(bytes, 1)?), 5)),
        _ => None,
    }
}

fn read_u16(bytes: &[u8], offset: &mut usize) -> Option<u16> {
    let marker = *bytes.get(*offset)?;
    *offset += 1;

    match marker {
        0x00..=0x7f => Some(u16::from(marker)),
        0xcc => {
            let value = *bytes.get(*offset)?;
            *offset += 1;
            Some(u16::from(value))
        }
        0xcd => {
            let value = u16::from_be_bytes(read_array(bytes, *offset)?);
            *offset += 2;
            Some(value)
        }
        _ => None,
    }
}

fn read_array<const N: usize>(bytes: &[u8], offset: usize) -> Option<[u8; N]> {
    bytes.get(offset..offset + N)?.try_into().ok()
}
