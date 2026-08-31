use super::super::LspWorkspace;

impl LspWorkspace {
    pub(crate) fn partition_kernel_generation(&self, id: &str) -> Option<u64> {
        self.partition(id)
            .map(|partition| partition.kernel.snapshot().generation().as_u64())
    }
}
