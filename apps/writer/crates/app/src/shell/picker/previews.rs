//! Passage snippets are prepared once per query for every match the picker lists.
pub use super::{Entry, Item};

#[derive(Default)]
pub struct Previews {
    prepared: bool,
    text: Vec<String>,
}
impl Previews {
    pub(crate) fn invalidate(&mut self) {
        self.prepared = false;
        self.text.clear();
    }
    pub fn prepare(&mut self, entries: &[Entry], matches: &[usize], query: &str) {
        if self.prepared {
            return;
        }
        self.text = matches
            .iter()
            .map(|index| match &entries[*index].item {
                Item::Passage { passage, .. } => {
                    recite_writer_model::passage_excerpt(&passage.text, query, 120)
                }
                _ => String::new(),
            })
            .collect();
        self.prepared = true;
    }
    pub fn get(&self, position: usize) -> Option<&str> {
        self.text.get(position).map(String::as_str)
    }
}
