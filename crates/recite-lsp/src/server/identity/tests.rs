use super::*;
impl UpdateRevision {
    pub(in crate::server) const fn for_test(value: u64) -> Self {
        Self(value)
    }
}
impl QuerySerial {
    pub(in crate::server) const fn for_test(value: u64) -> Self {
        Self(value)
    }
}
