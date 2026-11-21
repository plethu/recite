//! One reusable checker per project for long-lived authoring clients.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::SystemTime;

use crate::{ProseChecker, ProseSettings, ProseSetupError, dictionary};

type Stamp = Vec<Option<(SystemTime, u64)>>;

struct Entry {
    settings: ProseSettings,
    stamps: Stamp,
    checker: Arc<Mutex<ProseChecker>>,
}

/// Keeps a checker per project root and rebuilds it only when the project's
/// `[prose]` settings or dictionary files change. Editors share one cache
/// across worker threads.
#[derive(Default)]
pub struct ProseCheckerCache {
    entries: Mutex<BTreeMap<PathBuf, Entry>>,
}

impl std::fmt::Debug for ProseCheckerCache {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProseCheckerCache")
            .finish_non_exhaustive()
    }
}

impl ProseCheckerCache {
    /// The checker for a project, built on first use or after a change.
    pub fn checker(
        &self,
        project_root: &Path,
        settings: &ProseSettings,
    ) -> Result<Arc<Mutex<ProseChecker>>, ProseSetupError> {
        let stamps = dictionary::stamps(settings.dictionaries());
        let mut entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(entry) = entries.get(project_root)
            && entry.settings == *settings
            && entry.stamps == stamps
        {
            return Ok(entry.checker.clone());
        }
        let checker = Arc::new(Mutex::new(ProseChecker::new(settings.clone())?));
        entries.insert(
            project_root.to_owned(),
            Entry {
                settings: settings.clone(),
                stamps,
                checker: checker.clone(),
            },
        );
        Ok(checker)
    }
}
