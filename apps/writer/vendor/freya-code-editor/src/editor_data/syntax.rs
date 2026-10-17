//! Background syntax lifecycle and freshness for the authoritative editor Rope.
use super::*;

impl CodeEditorData {
    /// Use one bounded worker for full-tree parsing and visible syntax rows.
    /// Text, selection and history remain on the caller's thread.
    pub fn enable_background_highlighting(&mut self) -> std::io::Result<()> {
        if self.background.is_none() || self.syntax_failed {
            self.background = match crate::background::BackgroundSyntax::new() {
                Ok(worker) => Some(worker),
                Err(error) => {
                    self.fail_syntax();
                    return Err(error);
                }
            };
            self.syntax_failed = false;
            self.metrics.highlighter.invalidate_tree();
            self.metrics.syntax_blocks.clear();
            self.pending_edits = None;
            self.parse();
        }
        Ok(())
    }

    pub fn parse(&mut self) {
        self.observe_source();
        if self.syntax_failed {
            return;
        }
        if let Some(background) = &self.background {
            let stamp = std::sync::Arc::new(());
            self.syntax_pending = background.submit(crate::background::Request {
                base: self.submitted.clone(),
                stamp: std::sync::Arc::clone(&stamp),
                rope: self.rope.clone(),
                edits: self.pending_edits.take(),
                rows: self.visible_rows.clone(),
                language: self.language.clone(),
                theme: self.theme.clone(),
            });
            self.submitted = Some(stamp);
            self.submitted_source = Some(self.rope.clone());
            self.pending_edits = Some(Vec::new());
            if !self.syntax_pending {
                self.fail_syntax();
            }
        } else {
            self.syntax_pending = false;
            self.metrics.run_parser(&self.rope, None, &self.theme);
        }
    }

    pub(crate) fn syntax_notifications(&self) -> Option<async_channel::Receiver<()>> {
        if self.syntax_failed {
            return None;
        }
        self.background
            .as_ref()
            .map(|worker| worker.notifications())
    }

    pub(crate) fn receive_syntax(
        &mut self,
        receiver: &async_channel::Receiver<()>,
        closed: bool,
    ) -> bool {
        if !self
            .background
            .as_ref()
            .is_some_and(|worker| worker.notifications().same_channel(receiver))
        {
            return false;
        }
        if closed {
            self.fail_syntax();
            true
        } else {
            self.continue_syntax()
        }
    }

    pub(crate) fn continue_syntax(&mut self) -> bool {
        self.observe_source();
        let Some(background) = &self.background else {
            return false;
        };
        match background.take() {
            Some(Ok(output))
                if self
                    .submitted
                    .as_ref()
                    .is_some_and(|stamp| std::sync::Arc::ptr_eq(stamp, &output.stamp))
                    && self
                        .submitted_source
                        .as_ref()
                        .is_some_and(|source| source.is_instance(&self.rope))
                    && self.pending_edits.as_ref().is_some_and(Vec::is_empty) =>
            {
                self.metrics.syntax_blocks = output.blocks;
                self.syntax_pending = false;
                true
            }
            Some(Err(())) => {
                self.fail_syntax();
                true
            }
            _ => false,
        }
    }

    fn fail_syntax(&mut self) {
        self.invalidate_syntax();
        if !std::mem::replace(&mut self.syntax_failed, true) {
            eprintln!("Source highlighting unavailable; rendering uncoloured source.");
        }
    }

    pub(crate) fn visible_rows(&mut self, rows: Range<usize>) -> bool {
        if self.background.is_none() || self.visible_rows == rows {
            return false;
        }
        self.visible_rows = rows;
        self.metrics.syntax_blocks.retain_rows(&self.visible_rows);
        self.parse();
        false
    }

    pub(crate) fn syntax_text_color(&self) -> Color {
        self.theme.text
    }

    pub(super) fn record_edit(
        &mut self,
        edit: InputEdit,
        had_non_lf_breaks: bool,
        old_chars: Range<usize>,
        new_end_char: usize,
    ) {
        self.syntax_pending = false;
        self.tracked_source = Some(self.rope.clone());
        let cold = had_non_lf_breaks || self.metrics.line_widths.has_non_lf_breaks(&self.rope);
        if self.background.is_some() {
            if cold {
                self.invalidate_syntax();
            } else {
                self.metrics
                    .syntax_blocks
                    .relocate(&edit, old_chars, new_end_char);
                if let Some(edits) = &mut self.pending_edits {
                    if edits.len() < crate::background::MAX_EDITS {
                        edits.push(edit);
                    } else {
                        self.pending_edits = None;
                    }
                }
            }
        } else {
            if cold {
                self.metrics.highlighter.invalidate_tree();
            } else {
                self.metrics.highlighter.edit(&edit);
            }
            self.metrics.syntax_blocks.clear();
        }
    }

    pub(super) fn observe_source(&mut self) {
        if self
            .tracked_source
            .as_ref()
            .is_some_and(|source| !source.is_instance(&self.rope))
        {
            self.invalidate_syntax();
        }
        self.tracked_source = Some(self.rope.clone());
    }

    pub(crate) fn syntax_rows_current(&self) -> bool {
        self.tracked_source
            .as_ref()
            .is_some_and(|source| source.is_instance(&self.rope))
    }

    pub(super) fn invalidate_syntax(&mut self) {
        self.syntax_pending = false;
        self.pending_edits = None;
        self.metrics.syntax_blocks.clear();
        if self.background.is_none() {
            self.metrics.highlighter.invalidate_tree();
        }
    }
}

#[cfg(test)]
mod tests;
