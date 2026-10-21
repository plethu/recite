//! Initial scene presentation preserves recovered editing targets before applying preferences.
use super::{Pane, Writer};
use crate::scene::has_no_beats;
use freya::prelude::*;
use recite_config::WriterView;
use recite_writer_model::{View, Workbench};

impl Writer {
    pub fn scene_opened(mut self, view: WriterView) -> Result<(), String> {
        if self.restore_pending_scene() {
            return Ok(());
        }
        if view == WriterView::Source {
            self.try_navigate(|m| m.select(View::Source))?;
        } else if !self.buffers.model.peek().as_ref().is_ok_and(has_no_beats) {
            self.try_navigate(Workbench::show_script)?;
        }
        self.selection.set(None);
        self.layout.view.set(view);
        if view != WriterView::Source
            && self.layout.standalone()
            && !self.buffers.model.peek().as_ref().is_ok_and(has_no_beats)
        {
            self.ensure_beat()?;
        }
        Ok(())
    }
    /// Restoring a captured editing target is presentation, not a request to leave it.
    fn restore_pending_scene(mut self) -> bool {
        let state = self.buffers.model.peek();
        let Ok(model) = state.as_ref() else {
            return false;
        };
        if !model.has_draft() {
            return false;
        }
        match model.view() {
            View::Source => {
                self.layout.view.set(WriterView::Source);
                self.pane.set(Pane::Map);
                self.selection.set(None);
            }
            View::Passage(_) | View::Block(_) => {
                self.layout.view.set(WriterView::Script);
                self.pane.set(Pane::Script);
                self.selection.set(model.selected_block().ok().flatten());
            }
        }
        true
    }
}

#[cfg(test)]
mod tests;
