use lsp_types::Uri;
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
#[derive(Default)]
pub(super) struct Versions(BTreeMap<Uri, i32>);
impl Versions {
    /// Validate full-sync input before it can invalidate requests or displace accepted text.
    pub(super) fn accept(&mut self, update: &super::updates::Update) -> bool {
        use super::updates::Update;
        match update {
            Update::Open(p) => {
                if self.0.contains_key(&p.text_document.uri) {
                    return false;
                }
                self.0
                    .insert(p.text_document.uri.clone(), p.text_document.version);
            }
            Update::Change(p) => {
                let Some(version) = self.0.get_mut(&p.text_document.uri) else {
                    return false;
                };
                let [change] = p.content_changes.as_slice() else {
                    return false;
                };
                if p.text_document.version <= *version
                    || change.range.is_some()
                    || change.range_length.is_some()
                {
                    return false;
                }
                *version = p.text_document.version;
            }
            Update::Close(p) => {
                return self.0.remove(&p.text_document.uri).is_some();
            }
            _ => {}
        }
        true
    }
}
