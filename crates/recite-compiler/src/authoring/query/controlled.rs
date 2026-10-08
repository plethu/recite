use crate::authoring::{AuthoringSnapshot, Interrupted, WorkControl};

/// A borrowed immutable snapshot paired with one host-owned work lifetime.
/// Cloning a snapshot never copies request cancellation into future queries.
pub struct AuthoringQuery<'a> {
    snapshot: &'a AuthoringSnapshot,
    control: &'a dyn WorkControl,
}
impl AuthoringSnapshot {
    /// Compose read-only operations under one cooperative interruption token.
    #[must_use]
    pub fn query<'a>(&'a self, control: &'a dyn WorkControl) -> AuthoringQuery<'a> {
        AuthoringQuery {
            snapshot: self,
            control,
        }
    }
}
impl AuthoringQuery<'_> {
    pub fn checkpoint(&self) -> Result<(), Interrupted> {
        self.control.checkpoint()
    }
}
impl std::ops::Deref for AuthoringQuery<'_> {
    type Target = AuthoringSnapshot;
    fn deref(&self) -> &Self::Target {
        self.snapshot
    }
}

#[cfg(test)]
mod tests;
