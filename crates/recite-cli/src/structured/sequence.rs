//! Record order belongs to a CLI output stream, independently of build freshness.
#[derive(Clone, Copy, Default, serde::Serialize)]
#[serde(transparent)]
pub(crate) struct RecordSequence(u64);
impl RecordSequence {
    pub(crate) fn advance(&mut self) {
        self.0 += 1;
    }
}
