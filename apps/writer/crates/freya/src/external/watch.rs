//! Filesystem notifications are advisory; checked writes remain authoritative.
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    path::{Path, PathBuf},
    sync::mpsc,
};
pub(crate) struct Watch {
    _watcher: RecommendedWatcher,
    events: mpsc::Receiver<notify::Result<notify::Event>>,
}
impl Watch {
    pub fn new(root: &Path) -> Result<Self, String> {
        Self::directory(root, RecursiveMode::Recursive)
    }
    pub fn parent(path: &Path) -> Result<Self, String> {
        Self::directory(
            path.parent().ok_or("File has no parent directory")?,
            RecursiveMode::NonRecursive,
        )
    }
    fn directory(root: &Path, mode: RecursiveMode) -> Result<Self, String> {
        let (send, events) = mpsc::sync_channel(64);
        let mut watcher = notify::recommended_watcher(move |event| {
            let _ = send.try_send(event);
        })
        .map_err(|e| e.to_string())?;
        watcher.watch(root, mode).map_err(|e| e.to_string())?;
        Ok(Self {
            _watcher: watcher,
            events,
        })
    }
    pub fn changed(&self) -> Result<Vec<PathBuf>, String> {
        let mut paths = std::collections::BTreeSet::new();
        for event in self.events.try_iter().take(64) {
            let event = event.map_err(|e| e.to_string())?;
            if matches!(
                event.kind,
                notify::EventKind::Modify(_)
                    | notify::EventKind::Create(_)
                    | notify::EventKind::Remove(_)
            ) {
                paths.extend(event.paths);
            }
        }
        Ok(paths.into_iter().collect())
    }
}
