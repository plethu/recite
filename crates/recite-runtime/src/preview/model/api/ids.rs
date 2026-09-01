/// Stable identity for one condition request in a preview session.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct PreviewConditionRequestId(u64);

impl PreviewConditionRequestId {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl From<u64> for PreviewConditionRequestId {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

impl std::fmt::Display for PreviewConditionRequestId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Caller-owned revision of the immutable inputs used for a traversal.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct PreviewInputRevision(u64);

impl PreviewInputRevision {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl From<u64> for PreviewInputRevision {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

impl std::fmt::Display for PreviewInputRevision {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}
