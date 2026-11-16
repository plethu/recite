//! What a native frontend lends the toolkit-independent workflows.
/// Native services a frontend lends to authoring workflows.
#[derive(Clone, Copy)]
pub struct Host {
    /// Ask the person for a repaired source file; runs on a worker thread.
    pub choose_repaired_source: fn() -> Option<std::path::PathBuf>,
}
impl Default for Host {
    fn default() -> Self {
        Self {
            choose_repaired_source: || None,
        }
    }
}
