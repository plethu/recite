//! Coalesced notifications are advisory; checked writes remain authoritative.
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

const PATHS: usize = 64;

#[derive(Default)]
pub struct Changes {
    pub affected: Affected,
    pub error: Option<String>,
}
impl Changes {
    pub fn merge(&mut self, other: Self) {
        self.affected.merge(other.affected);
        if self.error.is_none() {
            self.error = other.error;
        }
    }
}
#[derive(Clone)]
pub enum Affected {
    Paths(BTreeSet<PathBuf>),
    AllOpen,
}
impl Default for Affected {
    fn default() -> Self {
        Self::Paths(BTreeSet::new())
    }
}
impl Affected {
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Paths(paths) if paths.is_empty())
    }
    pub fn contains(&self, path: &Path) -> bool {
        match self {
            Self::Paths(paths) => paths.contains(path),
            Self::AllOpen => true,
        }
    }
    pub fn insert(&mut self, path: PathBuf) {
        if let Self::Paths(paths) = self {
            paths.insert(path);
            if paths.len() > PATHS {
                *self = Self::AllOpen;
            }
        }
    }
    pub fn merge(&mut self, other: Self) {
        match other {
            Self::AllOpen => *self = Self::AllOpen,
            Self::Paths(paths) => {
                for path in paths {
                    self.insert(path);
                }
            }
        }
    }
}
struct Inbox {
    pending: Arc<Mutex<Changes>>,
    wake: async_channel::Sender<()>,
}
pub struct Subscription {
    pending: Arc<Mutex<Changes>>,
    wake: async_channel::Receiver<()>,
}
fn channel() -> (Inbox, Subscription) {
    let pending = Arc::new(Mutex::new(Changes::default()));
    let (send, receive) = async_channel::bounded(1);
    (
        Inbox {
            pending: pending.clone(),
            wake: send,
        },
        Subscription {
            pending,
            wake: receive,
        },
    )
}
impl Inbox {
    fn accept(&self, event: notify::Result<notify::Event>) {
        let mut pending = self
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match event {
            Err(error) => {
                pending.error.get_or_insert_with(|| error.to_string());
            }
            Ok(event) if event.need_rescan() => pending.affected = Affected::AllOpen,
            Ok(event)
                if matches!(
                    event.kind,
                    notify::EventKind::Modify(_)
                        | notify::EventKind::Create(_)
                        | notify::EventKind::Remove(_)
                ) =>
            {
                for path in event.paths {
                    pending.affected.insert(path);
                }
            }
            _ => return,
        }
        // The batch owns the evidence; a full wake queue already schedules its consumer.
        let _ = self.wake.try_send(());
    }
}
impl Subscription {
    fn take(&self) -> Changes {
        std::mem::take(
            &mut *self
                .pending
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
    }
    pub async fn next(&self) -> Option<Changes> {
        while self.wake.recv().await.is_ok() {
            let changes = self.take();
            if !changes.affected.is_empty() || changes.error.is_some() {
                return Some(changes);
            }
        }
        None
    }
    pub fn changed(&self) -> Changes {
        if self.wake.try_recv().is_ok() {
            self.take()
        } else {
            Changes::default()
        }
    }
}
pub struct Watch {
    _watcher: RecommendedWatcher,
    source: Subscription,
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
        let (inbox, source) = channel();
        let mut watcher = notify::recommended_watcher(move |event| inbox.accept(event))
            .map_err(|e| e.to_string())?;
        watcher.watch(root, mode).map_err(|e| e.to_string())?;
        Ok(Self {
            _watcher: watcher,
            source,
        })
    }
    /// Choose one synchronous or asynchronous consumer per watcher; subscriptions share an inbox.
    pub fn subscribe(&self) -> Subscription {
        Subscription {
            pending: self.source.pending.clone(),
            wake: self.source.wake.clone(),
        }
    }
    pub fn changed(&self) -> Changes {
        self.source.changed()
    }
}

#[cfg(test)]
mod tests;
