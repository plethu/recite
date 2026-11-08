//! Bounded declaration inspection; the shared Session remains the canonical schema/source owner.
use super::*;
mod producer;
pub mod source;
pub const PAGE: usize = 5;
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Inspect,
    Source,
    Producer,
}
impl Writer {
    pub fn show_declaration_source(&mut self) {
        self.workspaces.declarations.mode = Mode::Source;
    }
    pub fn show_declaration_producer(&mut self) {
        self.workspaces.declarations.mode = Mode::Producer;
    }
}
pub struct Row {
    pub name: String,
    pub kind: String,
    folded: String,
    pub detail: Vec<String>,
}
impl Row {
    pub fn new((name, kind, detail): (String, String, String)) -> Self {
        let folded = format!("{name} {kind}").to_lowercase();
        Self {
            name,
            kind,
            folded,
            detail: super::text::pages(&detail),
        }
    }
}
#[derive(Default)]
pub struct Panel {
    pub mode: Mode,
    pub producer: producer::Panel,
    pub source: source::Panel,
    pub rows: Vec<Row>,
    pub query: String,
    pub matches: Vec<usize>,
    pub page: usize,
    pub selected: Option<usize>,
    pub detail_page: usize,
}
impl Panel {
    pub(crate) fn describe_producer(&mut self, pages: Vec<String>) {
        self.producer = producer::Panel { pages, page: 0 };
    }
    pub(crate) fn replace(&mut self, rows: Vec<Row>) {
        self.rows = rows;
        self.filter(self.query.clone());
    }
    pub fn filter(&mut self, query: String) {
        let needle = query.trim().to_lowercase();
        self.matches = self
            .rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| row.folded.contains(&needle).then_some(index))
            .collect();
        self.query = query;
        self.page = 0;
        self.selected = self.matches.first().copied();
        self.detail_page = 0;
    }
    pub fn choose(&mut self, name: &str, kind: &str) {
        self.selected = self
            .rows
            .iter()
            .position(|row| row.name == name && row.kind == kind);
        self.detail_page = 0;
    }
}
pub use producer::description as producer_description;
