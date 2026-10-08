use std::collections::BTreeMap;

#[derive(Clone, Default)]
pub(super) struct Epochs {
    topology: u64,
    revision: u64,
    partitions: BTreeMap<String, u64>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Fence {
    topology: u64,
    partition: Option<String>,
    epoch: u64,
}
impl Epochs {
    pub(super) fn fence(&self, partition: Option<&str>) -> Fence {
        Fence {
            topology: self.topology,
            partition: partition.map(str::to_owned),
            epoch: partition.map_or(self.revision, |p| {
                self.partitions.get(p).copied().unwrap_or(0)
            }),
        }
    }
    pub(super) fn same_topology(&self, other: &Self) -> bool {
        self.topology == other.topology
    }

    pub(super) fn matches(&self, fence: &Fence) -> bool {
        self.fence(fence.partition.as_deref()) == *fence
    }
    pub(super) fn advance(&mut self, partition: Option<&str>) -> Option<()> {
        self.revision = self.revision.checked_add(1)?;
        let epoch = match partition {
            Some(p) => self.partitions.entry(p.to_owned()).or_default(),
            None => &mut self.topology,
        };
        *epoch = epoch.checked_add(1)?;
        Some(())
    }
}
