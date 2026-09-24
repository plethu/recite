//! User configuration orchestration; loading and editing share one validator.
use super::{
    CONFIG_VERSION, ConfigError, LoadedUserConfig, UserConfigEdit,
    load::{load_source, parse_user_config},
    storage::UserFileTransaction,
};
use crate::{ResolvedConfigPath, production_config_path};

pub use super::storage::ConfigWriteError;

/// The resolved user configuration authority. It never discovers project files.
#[derive(Clone, Debug)]
pub struct UserConfigStore {
    path: Option<ResolvedConfigPath>,
}

impl UserConfigStore {
    /// Uses the same platform/environment resolution as `load_user_config`.
    pub fn discover() -> Result<Self, ConfigError> {
        Ok(Self::new(production_config_path()?))
    }

    /// Uses an already resolved path; useful for explicit configuration and tests.
    #[must_use]
    pub const fn new(path: Option<ResolvedConfigPath>) -> Self {
        Self { path }
    }

    /// A separate user-owned state file beside the resolved configuration.
    /// The caller owns its format and validation; configuration contents are untouched.
    pub fn state_file(&self, name: &str) -> Result<super::UserStateFile, ConfigWriteError> {
        if name.is_empty()
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
            || name == "."
            || name == ".."
        {
            return Err(ConfigWriteError::StateName);
        }
        let parent = self
            .path
            .as_ref()
            .and_then(|p| p.path().parent())
            .ok_or(ConfigWriteError::NoPath)?;
        let state_path = parent.join(name);
        if self.path.as_ref().is_some_and(|p| {
            p.path()
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.eq_ignore_ascii_case(name))
        }) {
            return Err(ConfigWriteError::StateName);
        }
        Ok(super::UserStateFile::new(state_path))
    }

    /// Resolved user configuration file, when the platform provides one.
    pub fn path(&self) -> Option<&std::path::Path> {
        self.path.as_ref().map(|p| p.path())
    }

    /// Read-only load, with the established defaults, strictness, and provenance.
    pub fn load(&self) -> Result<LoadedUserConfig, ConfigError> {
        super::load_user_config_path(self.path.as_ref())
    }

    /// Reloads under a cooperative lock, edits one typed setting, validates the
    /// result with the ordinary loader, and atomically replaces the file.
    /// Unrelated settings and comments survive, including external edits made
    /// since an earlier `load`. Missing explicit overrides remain errors.
    pub fn update(&self, edit: UserConfigEdit) -> Result<LoadedUserConfig, ConfigWriteError> {
        let path = self.path.as_ref().ok_or(ConfigWriteError::NoPath)?;
        let transaction = UserFileTransaction::begin(path.path())?;
        let (previous, source) = load_source(Some(path))?;
        let initial = source
            .clone()
            .unwrap_or_else(|| format!("config_version = {CONFIG_VERSION}\n"));
        let mut document = initial.parse::<toml_edit::DocumentMut>()?;
        edit.apply(&mut document);
        let replacement = document.to_string();
        let loaded = parse_user_config(&replacement, path.path(), previous.provenance)?;
        transaction.replace(source.as_deref(), &replacement)?;
        Ok(loaded)
    }
}
