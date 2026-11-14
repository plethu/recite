//! Typed bilingual records reuse the manuscript's structural reading projection.
pub use super::{EntryKey, Owner, text};
pub use crate::reading::{Content, PassageNumber, Row as ReadingRow};
use recite_writer_model::{AnalysisStamp, Passage};
use recite_writer_session::localisation::catalogue::EntryDetails;
use std::sync::Arc;

pub enum Row {
    Passage(PassageRow),
    Context(Context),
}
pub struct PassageRow {
    pub source: Passage,
    pub number: PassageNumber,
    pub reply: Option<usize>,
    pub groups: Arc<[String]>,
    pub pages: Vec<String>,
    pub source_page: usize,
    pub form_page: usize,
    pub translation: Option<Translation>,
}
pub struct Translation {
    pub key: EntryKey,
    pub details: Arc<EntryDetails>,
}
pub struct Context {
    pub index: usize,
    pub groups: Arc<[String]>,
    pub pages: Vec<String>,
    pub page: usize,
    /// A continuation line, which the frontend sets in interface type so its arrow renders.
    pub continuation: bool,
}
#[derive(Clone, PartialEq)]
pub enum RowIdentity {
    Passage(String),
    Structural(AnalysisStamp, usize),
}
impl Row {
    pub fn prepare(owner: Option<&Owner>, row: &ReadingRow, index: usize) -> Self {
        match &row.content {
            Content::Passage { passage, number } => Self::Passage(PassageRow {
                source: passage.clone(),
                number: *number,
                reply: row.reply,
                groups: row.groups.clone(),
                pages: text::reading_pages(&passage.text),
                source_page: 0,
                form_page: 0,
                translation: owner.and_then(|owner| {
                    let id = owner.catalogue.entry_for(&passage.id, &passage.text)?;
                    Some(Translation {
                        key: EntryKey {
                            catalogue: owner.identity.clone(),
                            id,
                        },
                        details: owner.catalogue.entry_details(id)?,
                    })
                }),
            }),
            Content::Heading(text) | Content::Note(text) => Self::context(row, index, text),
            Content::Continuation(target) => {
                let mut context = Self::context(row, index, &format!("Continue → {target}"));
                if let Self::Context(context) = &mut context {
                    context.continuation = true;
                }
                context
            }
        }
    }
    pub fn context(row: &ReadingRow, index: usize, text: &str) -> Self {
        Self::Context(Context {
            index,
            groups: row.groups.clone(),
            pages: text::reading_pages(text),
            page: 0,
            continuation: false,
        })
    }
    pub fn passage(&self) -> Option<&PassageRow> {
        match self {
            Self::Passage(row) => Some(row),
            Self::Context(_) => None,
        }
    }
    pub(crate) fn passage_mut(&mut self) -> Option<&mut PassageRow> {
        match self {
            Self::Passage(row) => Some(row),
            Self::Context(_) => None,
        }
    }
    pub fn identity(&self, source: &AnalysisStamp) -> RowIdentity {
        match self {
            Self::Passage(row) => RowIdentity::Passage(row.source.id.clone()),
            Self::Context(row) => RowIdentity::Structural(source.clone(), row.index),
        }
    }
}
