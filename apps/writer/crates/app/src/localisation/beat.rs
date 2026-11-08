//! A bounded bilingual beat projects source order without owning translation drafts.
pub mod incoming;
pub mod rows;
pub use super::{EntryKey, Localisation, Writer, slot::Owner, text};
pub use crate::reading::{self, Content};
use incoming::Incoming;
use recite_core::po::PoEntryId;
use recite_writer_model::{AnalysisStamp, Workbench};
pub use rows::{Context, PassageRow, Row, Translation};

pub const PAGE: usize = 8;
pub struct Selection {
    pub name: Option<String>,
    pub anchor: Option<String>,
    pub page: usize,
}
pub struct Beat {
    pub name: String,
    pub source: AnalysisStamp,
    pub rows: Vec<Row>,
    pub incoming: Incoming,
    pub total: usize,
    pub page: usize,
}
impl Beat {
    pub fn is_current(&self, model: &Workbench) -> bool {
        self.source.is_current(model.document())
    }
    pub fn prepare(
        owner: Option<&Owner>,
        model: &Workbench,
        selection: &Selection,
    ) -> Result<Self, String> {
        let passages = model.document().cached_passages().unwrap_or_default();
        let selected = model
            .selected_block()
            .map_err(|error| error.to_string())?
            .or_else(|| model.remembered_script_block().map(str::to_owned));
        let anchor = selection.anchor.as_deref();
        let name = match anchor {
            Some(anchor) => passages
                .iter()
                .find(|passage| passage.id == anchor)
                .map(|passage| passage.section.clone())
                .ok_or("The linked source passage is unavailable.")?,
            None => selection
                .name
                .clone()
                .or(selected)
                .ok_or("Choose a beat to translate.")?,
        };
        model
            .document()
            .validate_script_block(&name)
            .map_err(|error| error.to_string())?;
        let script = model
            .document()
            .script_snapshot()
            .map_err(|error| error.to_string())?;
        let block = script
            .iter()
            .find(|block| block.id == name)
            .ok_or("The selected beat is unavailable.")?;
        let reading = reading::flatten(&block.entries);
        let total = reading.len();
        let page = anchor.and_then(|anchor| reading.iter().position(|row| matches!(&row.content, Content::Passage { passage, .. } if passage.id == anchor)))
            .map_or(selection.page, |index| index / PAGE).min(total.saturating_sub(1) / PAGE);
        let rows = reading
            .iter()
            .enumerate()
            .skip(page * PAGE)
            .take(PAGE)
            .map(|(index, row)| Row::prepare(owner, row, index))
            .collect();
        Ok(Self {
            name: name.clone(),
            source: model.document().analysis_stamp(),
            incoming: Incoming::prepare(model, &name),
            rows,
            total,
            page,
        })
    }
}
impl Localisation {
    pub fn mounted_entries(&self) -> Vec<PoEntryId> {
        self.entry
            .iter()
            .map(|entry| entry.key.id)
            .chain(
                self.passage
                    .iter()
                    .flat_map(|beat| &beat.rows)
                    .filter_map(Row::passage)
                    .filter_map(|row| {
                        row.translation
                            .as_ref()
                            .map(|translation| translation.key.id)
                    }),
            )
            .collect()
    }
    pub(super) fn active_entry(&self, key: &EntryKey) -> bool {
        if self.slot.identity() != Some(&key.catalogue) {
            return false;
        }
        match self.destination {
            super::Destination::Entry => self.entry.as_ref().is_some_and(|entry| entry.key == *key),
            super::Destination::Passage => self.passage.as_ref().is_some_and(|beat| {
                beat.rows.iter().filter_map(Row::passage).any(|row| {
                    row.translation
                        .as_ref()
                        .is_some_and(|translation| translation.key == *key)
                })
            }),
            _ => false,
        }
    }
}
impl Writer {
    pub(super) fn translation_entry_active(&self, key: &EntryKey) -> bool {
        self.phase == super::Phase::Writing
            && self.workspaces.screen == super::Screen::Localisation
            && self.localisation.active_entry(key)
            && (!matches!(self.localisation.destination, super::Destination::Passage)
                || self
                    .localisation
                    .passage
                    .as_ref()
                    .is_some_and(|beat| beat.is_current(self.session.model())))
    }
    pub fn open_translation_beat(&mut self, page: usize) {
        if let Err(error) = self.show_current_catalogue_beat(page) {
            self.catalogue_feedback(error);
        }
    }
    pub fn translation_row_mut(&mut self, anchor: &str) -> Option<&mut PassageRow> {
        self.localisation
            .passage
            .as_mut()?
            .rows
            .iter_mut()
            .filter_map(Row::passage_mut)
            .find(|row| row.source.id == anchor)
    }
}

impl Localisation {
    /// The mounted source-context row at `index` in the open passage, if any.
    pub fn context_row_mut(&mut self, index: usize) -> Option<&mut Context> {
        self.passage
            .as_mut()?
            .rows
            .iter_mut()
            .find_map(|row| match row {
                Row::Context(row) if row.index == index => Some(row),
                _ => None,
            })
    }
}
