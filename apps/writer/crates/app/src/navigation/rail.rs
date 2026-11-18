//! Reveal accepted destinations in the reading view; the scene list scrolls instead of paging.
pub use crate::Writer;

impl Writer {
    pub(crate) fn reveal_navigation(&mut self) {
        self.reading.reveal_selected();
    }
}
