use compact_str::CompactString;
use std::fmt;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};

use crate::{ChoiceId, CoreValueError, LineId, STABLE_ID_FIELD_PREFIX};

/// Length of a tool-generated stable ID.
pub const GENERATED_STABLE_ID_LEN: usize = 7;

/// Lowercase Crockford base32 without `i`, `l`, `o` or `u`.
const STABLE_ID_ALPHABET: &[u8; 32] = b"0123456789abcdefghjkmnpqrstvwxyz";
/// Generated IDs start with a letter so they satisfy the authored-ID grammar.
const STABLE_ID_LETTERS: &[u8; 22] = b"abcdefghjkmnpqrstvwxyz";

/// The stable-ID state of one line or choice header.
///
/// Source spells a frozen ID as a trailing `@id` field. A header without one
/// is [`SourceId::Missing`], which the compiler rejects until tooling or the
/// author assigns an ID.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceId {
    Missing,
    Frozen(StableId),
    Malformed { raw: String },
}

impl SourceId {
    /// Whether a header field is spelled as a stable-ID field.
    #[must_use]
    pub fn is_field(field: &str) -> bool {
        field.starts_with(STABLE_ID_FIELD_PREFIX)
    }

    /// Parses the text of a trailing `@id` header field, including its `@`.
    #[must_use]
    pub fn parse_field(field: &str) -> Self {
        field
            .strip_prefix(STABLE_ID_FIELD_PREFIX)
            .and_then(|id| StableId::new(id).ok())
            .map_or_else(
                || Self::Malformed {
                    raw: field.to_owned(),
                },
                Self::Frozen,
            )
    }

    #[must_use]
    pub fn stable_id(&self) -> Option<&StableId> {
        match self {
            Self::Frozen(id) => Some(id),
            Self::Missing | Self::Malformed { .. } => None,
        }
    }

    /// Source spelling of the ID field, when the header has one.
    #[must_use]
    pub fn display_text(&self) -> Option<String> {
        match self {
            Self::Frozen(id) => Some(id.field().to_string()),
            Self::Malformed { raw } => Some(raw.clone()),
            Self::Missing => None,
        }
    }

    #[must_use]
    pub fn canonical_line_id(&self) -> Option<LineId> {
        self.stable_id()
            .and_then(|id| LineId::new(id.as_str()).ok())
    }

    #[must_use]
    pub fn canonical_choice_id(&self) -> Option<ChoiceId> {
        self.stable_id()
            .and_then(|id| ChoiceId::new(id.as_str()).ok())
    }
}

/// A line or choice identity, either author-chosen or tool-generated.
///
/// Both kinds share the grammar `[a-z_][a-z0-9_.-]*`. `&`, `:` and `/` are
/// excluded because localisation variants, PO context prefixes and future pack
/// qualifiers use them as separators.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct StableId(CompactString);

impl StableId {
    pub fn new(value: impl Into<String>) -> Result<Self, CoreValueError> {
        let value = value.into();
        if is_valid_stable_id(&value) {
            Ok(Self(CompactString::from(value)))
        } else {
            Err(CoreValueError::InvalidValue {
                kind: "StableId",
                value,
            })
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for StableId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Draws generated stable IDs.
///
/// Generated IDs are random rather than derived from source position, so
/// separate branches inserting at the same place do not produce the same ID.
/// Callers still check each candidate against the project namespace. Tests and
/// fixture generators use [`StableIdGenerator::seeded`] for reproducible output.
#[derive(Clone, Debug)]
pub struct StableIdGenerator {
    state: u64,
}

impl StableIdGenerator {
    /// Seeds from the process's OS-provided hashing keys.
    #[must_use]
    pub fn from_system() -> Self {
        let mut hasher = RandomState::new().build_hasher();
        hasher.write_u64(0);
        Self::seeded(hasher.finish())
    }

    #[must_use]
    pub const fn seeded(seed: u64) -> Self {
        Self { state: seed }
    }

    #[must_use]
    pub fn next_id(&mut self) -> StableId {
        let letters = STABLE_ID_LETTERS.len() as u64;
        let mut bits = self.next_u64();
        let mut id = CompactString::with_capacity(GENERATED_STABLE_ID_LEN);
        id.push(char::from(STABLE_ID_LETTERS[(bits % letters) as usize]));
        bits /= letters;
        for _ in 1..GENERATED_STABLE_ID_LEN {
            id.push(char::from(STABLE_ID_ALPHABET[(bits & 0x1f) as usize]));
            bits >>= 5;
        }
        StableId(id)
    }

    /// SplitMix64: small, well distributed and adequate for non-secret IDs.
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }
}

#[must_use]
pub fn is_valid_stable_id(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes
        .next()
        .is_some_and(|first| first.is_ascii_lowercase() || first == b'_')
        && bytes.all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'.' | b'-')
        })
}

/// Grammar for author-visible source names such as block names.
#[must_use]
pub fn is_valid_source_name(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (unicode_ident::is_xid_start(first) || first == '_')
        && chars.all(|character| {
            unicode_ident::is_xid_continue(character) || matches!(character, '_' | '-' | '.')
        })
}
