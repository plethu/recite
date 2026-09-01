use super::super::LspWorkspace;

impl LspWorkspace {
    pub(crate) fn partition_kernel_generation(&self, id: &str) -> Option<u64> {
        self.partition(id).map(|partition| partition.build_id)
    }

    pub(crate) fn partition_project_complete(&self, id: &str) -> Option<bool> {
        self.partition(id)
            .map(|partition| partition.kernel.project_complete())
    }
}
