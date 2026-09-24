//! One live user preference session shared by settings, navigation and closing.
use recite_config::{UserConfig, UserConfigEdit, UserConfigStore};

pub(crate) struct Preferences {
    pub config: UserConfig,
    pub error: Option<String>,
    store: Option<Result<UserConfigStore, String>>,
    pub path: String,
}
impl Preferences {
    pub fn load(store: Option<Result<UserConfigStore, String>>) -> Self {
        let loaded = store.as_ref().map(|s| {
            s.as_ref()
                .map_err(Clone::clone)
                .and_then(|s| s.load().map_err(|e| e.to_string()))
        });
        let config = loaded
            .as_ref()
            .and_then(|r| r.as_ref().ok())
            .map(|l| l.config.clone())
            .unwrap_or_default();
        let error = loaded.and_then(Result::err);
        let path = store
            .as_ref()
            .and_then(|s| s.as_ref().ok())
            .and_then(|s| s.path())
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "Session preferences".into());
        Self {
            config,
            error,
            store,
            path,
        }
    }
    pub fn update(&mut self, edit: UserConfigEdit) -> Result<(), String> {
        let outcome = match &self.store {
            Some(Ok(store)) => store
                .update(edit)
                .map(|l| self.config = l.config)
                .map_err(|e| e.to_string()),
            Some(Err(error)) => Err(error.clone()),
            None => {
                edit.apply_to(&mut self.config);
                Ok(())
            }
        };
        self.error = outcome.as_ref().err().cloned();
        outcome
    }
}
