//! Local server ordering, independent of authoring snapshot and freshness epochs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct UpdateRevision(u64);
impl UpdateRevision {
    pub(super) const fn as_u64(self) -> u64 {
        self.0
    }
    pub(super) fn checked_next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}
impl std::fmt::Display for UpdateRevision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct QuerySerial(u64);
impl QuerySerial {
    pub(super) const fn as_u64(self) -> u64 {
        self.0
    }
    pub(super) fn checked_next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}
impl std::fmt::Display for QuerySerial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests;
