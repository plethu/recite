//! Bounded declaration inspection; the shared Session remains the canonical schema/source owner.
use super::*;
use recite_writer_session::declarations::entries;
mod producer;
pub mod source;
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
/// What the sheet shows for the chosen declaration.
pub enum Detail {
    /// Parameter names and their author-facing types, in declaration order.
    Parameters(Vec<(String, String)>),
    /// Prose, bounded into pages so an enormous display name never mounts at once.
    Pages(Vec<String>),
}
pub struct Row {
    pub name: String,
    pub kind: String,
    folded: String,
    pub detail: Detail,
}
impl Row {
    pub fn new((name, kind, detail): (String, String, entries::Detail)) -> Self {
        let folded = format!("{name} {kind}").to_lowercase();
        Self {
            name,
            kind,
            folded,
            detail: match detail {
                entries::Detail::Parameters(parameters) => Detail::Parameters(parameters),
                entries::Detail::Text(text) => Detail::Pages(super::text::pages(&text)),
            },
        }
    }
    /// The prose pages, or none for a parameter list.
    pub fn pages(&self) -> &[String] {
        match &self.detail {
            Detail::Parameters(_) => &[],
            Detail::Pages(pages) => pages,
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
    /// Where the chosen declaration sits among the matches, so the list can scroll to it.
    pub fn chosen_position(&self) -> usize {
        self.selected
            .and_then(|selected| self.matches.iter().position(|index| *index == selected))
            .unwrap_or_default()
    }
}
pub use producer::description as producer_description;
