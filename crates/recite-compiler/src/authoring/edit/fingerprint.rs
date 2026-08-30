use std::fmt;

/// An opaque fingerprint of the complete source text used to plan edits.
///
/// The value is intentionally in-process only. Hosts can compare it with a
/// later [`SourceFingerprint::for_source`] result, but must not persist or
/// interpret its representation as a compatibility format.
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct SourceFingerprint([u8; 32]);

impl SourceFingerprint {
    /// Computes the in-process fingerprint used by edit preconditions.
    #[must_use]
    pub fn for_source(source: &str) -> Self {
        let fingerprint = recite_core::canonical_source_fingerprint(source);
        let mut digest = [0; 32];
        digest.copy_from_slice(fingerprint.digest().as_bytes());
        Self(digest)
    }

    /// Returns whether this fingerprint was computed from the supplied text.
    #[must_use]
    pub fn matches_source(&self, source: &str) -> bool {
        self == &Self::for_source(source)
    }
}

impl fmt::Debug for SourceFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SourceFingerprint(<opaque>)")
    }
}
