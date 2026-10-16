//! Authoritative text, selection and history mutations for Freya's editor.
use super::*;

impl TextEditor for CodeEditorData {
    type LinesIterator<'a>
        = LinesIterator<'a>
    where
        Self: 'a;

    fn lines(&self) -> Self::LinesIterator<'_> {
        unimplemented!("Unused.")
    }

    fn text(&self) -> Cow<'_, str> {
        self.rope.slice(..).into()
    }

    fn insert_char(&mut self, ch: char, idx: usize) -> usize {
        self.observe_source();
        let idx_utf8 = self.utf16_cu_to_char(idx);
        let selection = self.selection.clone();

        // Capture byte offset and position before mutation for InputEdit.
        let start_byte = self.rope.char_to_byte(idx_utf8);
        let start_line = self.rope.char_to_line(idx_utf8);
        let start_line_byte = self.rope.line_to_byte(start_line);
        let start_col = start_byte - start_line_byte;

        let len_before_insert = self.rope.len_utf16_cu();
        if self.background.is_none() {
            self.metrics.highlighter.reset_parse();
        }
        let had_non_lf_breaks = self.metrics.line_widths.has_non_lf_breaks(&self.rope);
        let first_width_row = start_line.saturating_sub(1);
        let last_width_row = (start_line + 1).min(self.rope.len_lines() - 1);
        self.metrics
            .line_widths
            .remove(&self.rope, first_width_row..=last_width_row);
        self.rope.insert_char(idx_utf8, ch);
        let len_after_insert = self.rope.len_utf16_cu();

        let inserted_text_len = len_after_insert - len_before_insert;

        // Compute new end position after insertion.
        let new_end_char = idx_utf8 + 1; // one char inserted
        let new_end_byte = self.rope.char_to_byte(new_end_char);
        let new_end_line = self.rope.char_to_line(new_end_char);
        let new_end_line_byte = self.rope.line_to_byte(new_end_line);
        let new_end_col = new_end_byte - new_end_line_byte;

        let last_width_row = (new_end_line + 1).min(self.rope.len_lines() - 1);
        self.metrics
            .line_widths
            .insert(&self.rope, first_width_row..=last_width_row);
        self.record_edit(
            InputEdit::new_edit(
                start_byte,
                start_byte,
                new_end_byte,
                (start_line, start_col),
                (start_line, start_col),
                (new_end_line, new_end_col),
            ),
            had_non_lf_breaks,
            idx_utf8..idx_utf8,
            new_end_char,
        );

        self.history.push_change(HistoryChange::InsertChar {
            idx,
            ch,
            len: inserted_text_len,
            selection,
        });

        inserted_text_len
    }

    fn insert(&mut self, text: &str, idx: usize) -> usize {
        self.observe_source();
        let idx_utf8 = self.utf16_cu_to_char(idx);
        let selection = self.selection.clone();

        // Capture byte offset and position before mutation for InputEdit.
        let start_byte = self.rope.char_to_byte(idx_utf8);
        let start_line = self.rope.char_to_line(idx_utf8);
        let start_line_byte = self.rope.line_to_byte(start_line);
        let start_col = start_byte - start_line_byte;

        let len_before_insert = self.rope.len_utf16_cu();
        if self.background.is_none() {
            self.metrics.highlighter.reset_parse();
        }
        let had_non_lf_breaks = self.metrics.line_widths.has_non_lf_breaks(&self.rope);
        let first_width_row = start_line.saturating_sub(1);
        let last_width_row = (start_line + 1).min(self.rope.len_lines() - 1);
        self.metrics
            .line_widths
            .remove(&self.rope, first_width_row..=last_width_row);
        self.rope.insert(idx_utf8, text);
        let len_after_insert = self.rope.len_utf16_cu();

        let inserted_text_len = len_after_insert - len_before_insert;

        // Compute new end position after insertion.
        let inserted_chars = text.chars().count();
        let new_end_char = idx_utf8 + inserted_chars;
        let new_end_byte = self.rope.char_to_byte(new_end_char);
        let new_end_line = self.rope.char_to_line(new_end_char);
        let new_end_line_byte = self.rope.line_to_byte(new_end_line);
        let new_end_col = new_end_byte - new_end_line_byte;

        let last_width_row = (new_end_line + 1).min(self.rope.len_lines() - 1);
        self.metrics
            .line_widths
            .insert(&self.rope, first_width_row..=last_width_row);
        self.record_edit(
            InputEdit::new_edit(
                start_byte,
                start_byte,
                new_end_byte,
                (start_line, start_col),
                (start_line, start_col),
                (new_end_line, new_end_col),
            ),
            had_non_lf_breaks,
            idx_utf8..idx_utf8,
            new_end_char,
        );

        self.history.push_change(HistoryChange::InsertText {
            idx,
            text: text.to_owned(),
            len: inserted_text_len,
            selection,
        });

        inserted_text_len
    }

    fn remove(&mut self, range_utf16: Range<usize>) -> usize {
        self.observe_source();
        let range =
            self.utf16_cu_to_char(range_utf16.start)..self.utf16_cu_to_char(range_utf16.end);
        let removed_chars = range.clone();
        let text = self.rope.slice(range.clone()).to_string();
        let selection = self.selection.clone();

        // Capture byte offsets and positions before mutation for InputEdit.
        let start_byte = self.rope.char_to_byte(range.start);
        let old_end_byte = self.rope.char_to_byte(range.end);
        let start_line = self.rope.char_to_line(range.start);
        let start_line_byte = self.rope.line_to_byte(start_line);
        let start_col = start_byte - start_line_byte;
        let old_end_line = self.rope.char_to_line(range.end);
        let old_end_line_byte = self.rope.line_to_byte(old_end_line);
        let old_end_col = old_end_byte - old_end_line_byte;

        let len_before_remove = self.rope.len_utf16_cu();
        if self.background.is_none() {
            self.metrics.highlighter.reset_parse();
        }
        let had_non_lf_breaks = self.metrics.line_widths.has_non_lf_breaks(&self.rope);
        let first_width_row = start_line.saturating_sub(1);
        let last_width_row = (old_end_line + 1).min(self.rope.len_lines() - 1);
        self.metrics
            .line_widths
            .remove(&self.rope, first_width_row..=last_width_row);
        self.rope.remove(range);
        let len_after_remove = self.rope.len_utf16_cu();

        let removed_text_len = len_before_remove - len_after_remove;

        let new_end_line = self.rope.char_to_line(self.rope.byte_to_char(start_byte));
        let last_width_row = (new_end_line + 1).min(self.rope.len_lines() - 1);
        self.metrics
            .line_widths
            .insert(&self.rope, first_width_row..=last_width_row);
        // After removal, new_end == start (the removed range collapses to a point).
        self.record_edit(
            InputEdit::new_edit(
                start_byte,
                old_end_byte,
                start_byte,
                (start_line, start_col),
                (old_end_line, old_end_col),
                (start_line, start_col),
            ),
            had_non_lf_breaks,
            removed_chars.clone(),
            removed_chars.start,
        );

        self.history.push_change(HistoryChange::Remove {
            idx: range_utf16.end - removed_text_len,
            text,
            len: removed_text_len,
            selection,
        });

        removed_text_len
    }

    fn char_to_line(&self, char_idx: usize) -> usize {
        self.rope.char_to_line(char_idx)
    }

    fn line_to_char(&self, line_idx: usize) -> usize {
        self.rope.line_to_char(line_idx)
    }

    fn utf16_cu_to_char(&self, utf16_cu_idx: usize) -> usize {
        self.rope.utf16_cu_to_char(utf16_cu_idx)
    }

    fn char_to_utf16_cu(&self, idx: usize) -> usize {
        self.rope.char_to_utf16_cu(idx)
    }

    fn line(&self, line_idx: usize) -> Option<Line<'_>> {
        let line = self.rope.get_line(line_idx);

        line.map(|line| Line {
            text: Cow::Owned(line.to_string()),
            utf16_len: line.len_utf16_cu(),
        })
    }

    fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    fn len_utf16_cu(&self) -> usize {
        self.rope.len_utf16_cu()
    }

    fn has_any_selection(&self) -> bool {
        self.selection.is_range()
    }

    fn get_selection(&self) -> Option<(usize, usize)> {
        match self.selection {
            TextSelection::Cursor(_) => None,
            TextSelection::Range { from, to } => Some((from, to)),
        }
    }

    fn set(&mut self, text: &str) {
        self.invalidate_syntax();
        self.rope.remove(0..);
        self.rope.insert(0, text);
    }

    fn clear_selection(&mut self) {
        let end = self.selection().end();
        self.selection_mut().set_as_cursor();
        self.selection_mut().move_to(end);
    }

    fn set_selection(&mut self, (from, to): (usize, usize)) {
        self.selection = TextSelection::Range { from, to };
    }

    fn get_selected_text(&self) -> Option<String> {
        let (start, end) = self.get_selection_range()?;

        Some(self.rope.get_slice(start..end)?.to_string())
    }

    fn get_selection_range(&self) -> Option<(usize, usize)> {
        let (start, end) = match self.selection {
            TextSelection::Cursor(_) => return None,
            TextSelection::Range { from, to } => (from, to),
        };

        // Use left-to-right selection
        let (start, end) = if start < end {
            (start, end)
        } else {
            (end, start)
        };

        Some((start, end))
    }

    fn undo(&mut self) -> Option<TextSelection> {
        if !self.history.can_undo() {
            return None;
        }
        // Undo can make arbitrary changes, therefore invalidate the tree for a full re-parse.
        self.invalidate_syntax();
        self.history.undo(&mut self.rope)
    }

    fn redo(&mut self) -> Option<TextSelection> {
        if !self.history.can_redo() {
            return None;
        }
        // Redo can make arbitrary changes, therefore invalidate the tree for a full re-parse.
        self.invalidate_syntax();
        self.history.redo(&mut self.rope)
    }

    fn editor_history(&self) -> &EditorHistory {
        &self.history
    }

    fn editor_history_mut(&mut self) -> &mut EditorHistory {
        &mut self.history
    }

    fn selection(&self) -> &TextSelection {
        &self.selection
    }

    fn selection_mut(&mut self) -> &mut TextSelection {
        &mut self.selection
    }

    fn get_indentation(&self) -> u8 {
        4
    }
}
