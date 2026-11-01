/// Order of outputs observed by one adapter owner; independent of asset revisions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct OutputSequence(u64);
impl OutputSequence {
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
    pub(crate) fn advance(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}
