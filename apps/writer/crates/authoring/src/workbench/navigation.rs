//! Editing selection and the beat to restore after leaving Source.
use super::{View, Workbench, WorkbenchError};
use crate::{EditError, Passage};

impl Workbench {
    pub(super) fn restore_script_selection(&mut self) -> Result<(), WorkbenchError> {
        let sections = self.document.script_snapshot().unwrap_or_default();
        let block = self
            .script_selection
            .iter()
            .find(|id| sections.iter().any(|block| &block.id == *id))
            .cloned()
            .or_else(|| sections.first().map(|block| block.id.clone()));
        // The old draft was already committed; history may have removed its target.
        self.load(String::new().into());
        match block {
            Some(block) => self.select(View::Block(block)),
            None => self.select(View::Source),
        }
    }

    pub fn selected(&self) -> Result<Option<Passage>, WorkbenchError> {
        match &self.view {
            View::Source | View::Block(_) => Ok(None),
            View::Passage(id) => Ok(self.document.find_passage(id)?),
        }
    }
    pub fn select(&mut self, view: View) -> Result<(), WorkbenchError> {
        self.require_applied()?;
        self.install_view(view, super::SourceValidation::Freeform)
    }

    pub(crate) fn restore_draft(
        &mut self,
        view: View,
        draft: crate::DraftSnapshot,
        validation: super::SourceValidation,
    ) -> Result<(), WorkbenchError> {
        // Selection validation finishes before displacing any current field state.
        self.install_view(view, validation)?;
        self.set_draft_snapshot(draft);
        Ok(())
    }

    fn install_view(
        &mut self,
        view: View,
        validation: super::SourceValidation,
    ) -> Result<(), WorkbenchError> {
        let text = match &view {
            View::Source => self.document.source_snapshot().into(),
            View::Block(block) => {
                if !self.document.sections().contains(block) {
                    return Err(EditError::Destination.into());
                }
                String::new().into()
            }
            View::Passage(id) => self
                .document
                .find_passage(id)?
                .ok_or(EditError::MissingPassage)?
                .text
                .clone()
                .into(),
        };
        let block = match &view {
            View::Block(block) => Some(block.clone()),
            View::Passage(id) => self.document.find_passage(id)?.map(|p| p.section.clone()),
            View::Source => None,
        };
        if let Some(block) = block {
            self.script_selection = Some(block);
        }
        self.view = view;
        self.source_validation = validation;
        self.load(text);
        Ok(())
    }
    /// Enter a beat without executing any dialogue or effects.
    pub fn inspect_block(&mut self, block: &str) -> Result<(), WorkbenchError> {
        self.require_applied()?;
        self.document.validate_script_block(block)?;
        self.select(View::Block(block.into()))
    }

    pub fn show_script(&mut self) -> Result<(), WorkbenchError> {
        self.require_applied()?;
        if let Some(block) = self.remembered_script_block().map(str::to_owned) {
            return self.inspect_block(&block);
        }
        let block = self
            .document
            .script_snapshot()?
            .first()
            .ok_or(EditError::MissingPassage)?
            .id
            .clone();
        self.inspect_block(&block)
    }
    /// The accepted Script selection, retained while a separate Source draft is open.
    pub fn remembered_script_block(&self) -> Option<&str> {
        self.script_selection.as_deref().filter(|block| {
            self.document
                .sections()
                .iter()
                .any(|section| section == block)
        })
    }
    pub fn selected_block(&self) -> Result<Option<String>, WorkbenchError> {
        match &self.view {
            View::Block(block) => Ok(Some(block.clone())),
            View::Passage(_) => Ok(self.selected()?.map(|passage| passage.section.clone())),
            View::Source => Ok(None),
        }
    }
}
