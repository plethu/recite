//! Reveal accepted destinations without moving pages during ordinary rail browsing.
pub use crate::{Writer, map::Workspace};

pub const FILE_PAGE: usize = 4;
const BEAT_PAGE: usize = 8;
const UNPAGED: usize = 12;

pub fn beat_page_size(total: usize) -> usize {
    if total <= UNPAGED { UNPAGED } else { BEAT_PAGE }
}

impl Writer {
    pub(crate) fn reveal_navigation(&mut self) {
        self.reading.reveal_selected();
        self.reveal_rail();
    }
    pub(crate) fn reveal_rail(&mut self) {
        if let Some(project) = self.session.project()
            && let Some(index) = project
                .documents()
                .iter()
                .position(|document| document.path == project.current().path())
        {
            self.file_page = index / FILE_PAGE;
        }
        let blocks = self.session.model().document().block_summaries();
        let selected = if self.visible_workspace() == Workspace::Map {
            self.map_selected().or(self.reading.selected.as_deref())
        } else {
            self.reading.selected.as_deref()
        };
        if let Some(index) = blocks
            .iter()
            .position(|block| Some(block.id().as_str()) == selected)
        {
            self.beat_page = index / beat_page_size(blocks.len());
        }
    }
}
