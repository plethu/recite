use crate::Keymap;

/// Invocation-owned overrides. The type exposes only the already-settled
/// invocation override, keymap; presentation fields remain user-only.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InvocationOverrides {
    keymap: Option<Keymap>,
}

impl InvocationOverrides {
    /// Creates an invocation with no overrides.
    #[must_use]
    pub const fn new() -> Self {
        Self { keymap: None }
    }

    /// Sets the invocation-owned keymap override.
    #[must_use]
    pub const fn with_keymap(mut self, keymap: Keymap) -> Self {
        self.keymap = Some(keymap);
        self
    }

    /// Returns the optional invocation keymap override.
    #[must_use]
    pub const fn keymap(&self) -> Option<Keymap> {
        self.keymap
    }
}
