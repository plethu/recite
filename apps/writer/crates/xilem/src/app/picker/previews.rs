//! Only the visible passage snippets are prepared, at query and result-page boundaries.
use super::{Entry, Item, PAGE};

#[derive(Default)]
pub(super) struct Previews {
    page: Option<usize>,
    text: Vec<String>,
}
impl Previews {
    pub fn invalidate(&mut self) {
        self.page = None;
        self.text.clear();
    }
    pub fn prepare(&mut self, entries: &[Entry], matches: &[usize], query: &str, active: usize) {
        let page = active / PAGE;
        if self.page == Some(page) {
            return;
        }
        self.text = matches
            .iter()
            .skip(page * PAGE)
            .take(PAGE)
            .map(|index| match &entries[*index].item {
                Item::Passage { passage, .. } => {
                    recite_writer_model::passage_excerpt(&passage.text, query, 120)
                }
                _ => String::new(),
            })
            .collect();
        self.page = Some(page);
    }
    pub fn get(&self, position: usize) -> Option<&str> {
        (self.page == Some(position / PAGE))
            .then(|| self.text.get(position % PAGE))
            .flatten()
            .map(String::as_str)
    }
}
