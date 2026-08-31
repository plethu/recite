use super::{LspWorkspace, SnapshotGeneration};

impl LspWorkspace {
    pub(crate) fn exhaust_generation_for_test(&mut self) {
        self.generation = SnapshotGeneration(u64::MAX);
    }
}
