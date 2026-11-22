use super::Ink;

impl Ink {
    /// Colour regardless of the terminal, for tests of styled output.
    pub(crate) const fn coloured() -> Self {
        Self { color: true }
    }
}
