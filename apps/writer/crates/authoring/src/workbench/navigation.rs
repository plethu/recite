//! Editing selection and the beat to restore after leaving Source.
use super::{View, Workbench, WorkbenchError};
use crate::{EditError, Passage};

impl Workbench {
    pub(super) fn restore_script_selection(&mut self) -> Result<(), WorkbenchError> {
        let sections = self
            .document
            .script()
            .map(|blocks| blocks.into_iter().map(|b| b.id).collect::<Vec<_>>())
            .unwrap_or_default();
        let block = self
            .script_selection
            .iter()
            .find(|id| sections.contains(id))
            .cloned()
            .or_else(|| sections.first().cloned());
        // The old draft was already committed; history may have removed its target.
        self.load(String::new());
        match block {
            Some(block) => self.inspect_block(&block),
            None => self.select(View::Source),
        }
    }

    pub fn selected(&self) -> Result<Option<Passage>, WorkbenchError> {
        match &self.view {
            View::Source | View::Block(_) => Ok(None),
            View::Passage(id) => Ok(self.document.passages()?.into_iter().find(|p| &p.id == id)),
        }
    }
    pub fn select(&mut self, view: View) -> Result<(), WorkbenchError> {
        self.require_applied()?;
        let text = match &view {
            View::Source => self.document.source().to_owned(),
            View::Block(block) => {
                if !self.document.sections().contains(block) {
                    return Err(EditError::Destination.into());
                }
                String::new()
            }
            View::Passage(id) => {
                self.document
                    .passages()?
                    .into_iter()
                    .find(|p| &p.id == id)
                    .ok_or(EditError::MissingPassage)?
                    .text
            }
        };
        let block = match &view {
            View::Block(block) => Some(block.clone()),
            View::Passage(id) => self
                .document
                .passages()?
                .into_iter()
                .find(|p| &p.id == id)
                .map(|p| p.section),
            View::Source => None,
        };
        if let Some(block) = block {
            self.script_selection = Some(block);
        }
        self.view = view;
        self.load(text);
        Ok(())
    }
    /// Enter a beat without executing any dialogue or effects.
    pub fn inspect_block(&mut self, block: &str) -> Result<(), WorkbenchError> {
        self.select(View::Block(block.into()))
    }

    pub fn show_script(&mut self) -> Result<(), WorkbenchError> {
        self.require_applied()?;
        if let Some(block) = self
            .script_selection
            .as_ref()
            .filter(|block| self.document.sections().contains(block))
            .cloned()
        {
            return self.select(View::Block(block));
        }
        if let Some(passage) = self.document.passages()?.into_iter().next() {
            return self.select(View::Passage(passage.id));
        }
        let block = self
            .document
            .script()?
            .into_iter()
            .next()
            .ok_or(EditError::MissingPassage)?;
        self.select(View::Block(block.id))
    }
    pub fn selected_block(&self) -> Result<Option<String>, WorkbenchError> {
        match &self.view {
            View::Block(block) => Ok(Some(block.clone())),
            View::Passage(_) => Ok(self.selected()?.map(|passage| passage.section)),
            View::Source => Ok(None),
        }
    }
}
