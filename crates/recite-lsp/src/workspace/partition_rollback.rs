use std::collections::BTreeMap;

use super::kernel::KernelPartition;

pub(super) fn take_old_partitions(
    old_partitions: &mut Option<BTreeMap<String, KernelPartition>>,
) -> BTreeMap<String, KernelPartition> {
    let Some(partitions) = old_partitions.take() else {
        unreachable!("candidate construction consumed old partitions")
    };
    partitions
}
