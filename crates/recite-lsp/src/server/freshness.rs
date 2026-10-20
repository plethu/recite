use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct TopologyEpoch(u64);
impl TopologyEpoch {
    fn checked_next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ScopeEpoch(u64);
impl ScopeEpoch {
    fn checked_next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

#[derive(Clone, Default)]
pub(super) struct Epochs {
    topology: TopologyEpoch,
    revision: ScopeEpoch,
    partitions: BTreeMap<String, ScopeEpoch>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Fence {
    topology: TopologyEpoch,
    partition: Option<String>,
    epoch: ScopeEpoch,
}
impl Epochs {
    pub(super) fn fence(&self, partition: Option<&str>) -> Fence {
        Fence {
            topology: self.topology,
            partition: partition.map(str::to_owned),
            epoch: partition.map_or(self.revision, |p| {
                self.partitions.get(p).copied().unwrap_or_default()
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
        self.revision = self.revision.checked_next()?;
        if let Some(partition) = partition {
            let epoch = self.partitions.entry(partition.to_owned()).or_default();
            *epoch = epoch.checked_next()?;
        } else {
            self.topology = self.topology.checked_next()?;
        }
        Some(())
    }
}

#[cfg(test)]
mod tests;
