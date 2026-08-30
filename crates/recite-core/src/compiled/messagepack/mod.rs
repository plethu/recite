use std::fmt;
use std::io::Cursor;

use serde::Deserialize;

use crate::CoreValueError;

use super::{
    COMPILED_ASSET_FORMAT_VERSION_V0, COMPILER_COMPATIBILITY_VERSION_V0, CompiledDialogue,
    CompiledValueError,
};

use probe::messagepack_asset_format_versions;
use validate::{ValidationMode, validate_dialogue};
use wire::MsgDialogue;

/// Error returned when encoding a compiled dialogue into canonical v0 bytes.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompiledAssetEncodeError {
    UnsupportedFormat {
        format_version: u16,
        compiler_compatibility_version: u16,
    },
    InvalidDialogue(String),
    MessagePack(String),
}

impl fmt::Display for CompiledAssetEncodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedFormat {
                format_version,
                compiler_compatibility_version,
            } => write!(
                formatter,
                "unsupported compiled asset format {format_version} with compatibility version {compiler_compatibility_version}"
            ),
            Self::InvalidDialogue(reason) => {
                write!(formatter, "invalid compiled dialogue: {reason}")
            }
            Self::MessagePack(reason) => {
                write!(formatter, "failed to encode MessagePack: {reason}")
            }
        }
    }
}

impl std::error::Error for CompiledAssetEncodeError {}

/// Error returned when decoding public v0 compiled dialogue asset bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompiledAssetDecodeError {
    UnsupportedFormat {
        format_version: u16,
        compiler_compatibility_version: u16,
    },
    MalformedAsset(String),
}

impl fmt::Display for CompiledAssetDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedFormat {
                format_version,
                compiler_compatibility_version,
            } => write!(
                formatter,
                "unsupported compiled asset format {format_version} with compatibility version {compiler_compatibility_version}"
            ),
            Self::MalformedAsset(reason) => write!(formatter, "malformed compiled asset: {reason}"),
        }
    }
}

impl std::error::Error for CompiledAssetDecodeError {}

/// Decode deterministic v0 `.recitec` MessagePack bytes into a compiled dialogue asset.
pub fn decode_compiled_dialogue_messagepack(
    bytes: &[u8],
) -> Result<CompiledDialogue, CompiledAssetDecodeError> {
    if let Some((format_version, compiler_compatibility_version)) =
        messagepack_asset_format_versions(bytes)
        && (format_version != COMPILED_ASSET_FORMAT_VERSION_V0
            || compiler_compatibility_version != COMPILER_COMPATIBILITY_VERSION_V0)
    {
        return Err(CompiledAssetDecodeError::UnsupportedFormat {
            format_version,
            compiler_compatibility_version,
        });
    }

    let mut cursor = Cursor::new(bytes);
    let mut deserializer = rmp_serde::Deserializer::new(&mut cursor);
    let wire = MsgDialogue::deserialize(&mut deserializer)
        .map_err(|error| malformed(error.to_string()))?;
    let consumed = cursor.position() as usize;
    if consumed != bytes.len() {
        return Err(malformed(format!(
            "MessagePack asset has {} trailing bytes",
            bytes.len() - consumed
        )));
    }

    let dialogue = wire.try_into()?;
    validate_dialogue(&dialogue, ValidationMode::Decoded)?;
    Ok(dialogue)
}

/// Encode a compiled dialogue using the canonical v0 wire contract.
///
/// This is the sole encoder authority for compiled dialogue bytes. The
/// compiler delegates here, and callers may use the exact bytes for durable
/// content identity without introducing a second semantic serialization.
pub fn encode_compiled_dialogue_messagepack(
    dialogue: &CompiledDialogue,
) -> Result<Vec<u8>, CompiledAssetEncodeError> {
    if dialogue.header.format_version != COMPILED_ASSET_FORMAT_VERSION_V0
        || dialogue.header.compiler_compatibility_version != COMPILER_COMPATIBILITY_VERSION_V0
    {
        return Err(CompiledAssetEncodeError::UnsupportedFormat {
            format_version: dialogue.header.format_version,
            compiler_compatibility_version: dialogue.header.compiler_compatibility_version,
        });
    }
    validate_dialogue(dialogue, ValidationMode::Canonical)
        .map_err(CompiledAssetEncodeError::from)?;
    encode::serialize_messagepack(dialogue)
}

mod encode;
mod interpolation;
mod probe;
mod tags;
mod validate;
mod wire;

fn malformed(reason: String) -> CompiledAssetDecodeError {
    CompiledAssetDecodeError::MalformedAsset(reason)
}

impl From<CompiledValueError> for CompiledAssetDecodeError {
    fn from(error: CompiledValueError) -> Self {
        malformed(error.to_string())
    }
}

impl From<CompiledAssetDecodeError> for CompiledAssetEncodeError {
    fn from(error: CompiledAssetDecodeError) -> Self {
        match error {
            CompiledAssetDecodeError::UnsupportedFormat {
                format_version,
                compiler_compatibility_version,
            } => Self::UnsupportedFormat {
                format_version,
                compiler_compatibility_version,
            },
            CompiledAssetDecodeError::MalformedAsset(reason) => Self::InvalidDialogue(reason),
        }
    }
}

impl From<CoreValueError> for CompiledAssetDecodeError {
    fn from(error: CoreValueError) -> Self {
        malformed(error.to_string())
    }
}
