/// A logical output target, not a filesystem path.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub struct BuildTarget(String);
impl BuildTarget {
    pub fn new(value: impl Into<String>) -> Result<Self, BuildTargetError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(BuildTargetError::Empty);
        }
        Ok(Self(value))
    }
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Display for BuildTarget {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum BuildTargetError {
    #[error("build target must not be empty")]
    Empty,
}

/// Bytes produced before a host publisher is permitted to commit.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct BuildCandidate {
    target: BuildTarget,
    bytes: Vec<u8>,
}
impl BuildCandidate {
    #[must_use]
    pub fn new(target: BuildTarget, bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            target,
            bytes: bytes.into(),
        }
    }
    #[must_use]
    pub const fn target(&self) -> &BuildTarget {
        &self.target
    }
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}
