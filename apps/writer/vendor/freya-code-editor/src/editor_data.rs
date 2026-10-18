mod input;
mod syntax;

use std::{
    borrow::Cow,
    fmt::Display,
    ops::{Mul, Range},
    time::Duration,
};

use freya_components::scrollviews::ScrollController;
use freya_core::{elements::paragraph::ParagraphHolderInner, prelude::*};
use freya_edit::*;
use ropey::Rope;
use torin::geometry::Size2D;
use tree_sitter::InputEdit;

use crate::{
    editor_theme::EditorSyntaxTheme, languages::EditorLanguage, metrics::EditorMetrics,
    syntax::InputEditExt,
};

pub struct CodeEditorData {
    pub(crate) history: EditorHistory,
    pub rope: Rope,
    pub(crate) selection: TextSelection,
    pub(crate) last_saved_history_change: usize,
    pub(crate) metrics: EditorMetrics,
    pub(crate) dragging: TextDragging,
    background: Option<crate::background::BackgroundSyntax>,
    submitted: Option<std::sync::Arc<()>>,
    submitted_source: Option<Rope>,
    tracked_source: Option<Rope>,
    pending_edits: Option<Vec<InputEdit>>,
    visible_rows: Range<usize>,
    syntax_pending: bool,
    syntax_failed: bool,
    pub language: Option<EditorLanguage>,
    pub viewport: Size2D,
    theme: EditorSyntaxTheme,
}

impl CodeEditorData {
    /// Creates the editor data for the given [`Rope`] and language.
    pub fn new(rope: Rope, language: impl Into<Option<EditorLanguage>>) -> Self {
        let mut data = Self {
            rope,
            selection: TextSelection::new_cursor(0),
            history: EditorHistory::new(Duration::from_secs(1)),
            last_saved_history_change: 0,
            metrics: EditorMetrics::new(),
            dragging: TextDragging::default(),
            background: None,
            submitted: None,
            submitted_source: None,
            tracked_source: None,
            pending_edits: None,
            visible_rows: 0..64,
            syntax_pending: false,
            syntax_failed: false,
            language: language.into(),
            viewport: Size2D::default(),
            theme: EditorSyntaxTheme::default(),
        };
        data.configure_highlighter();
        data
    }

    /// Scrolls the given controller just enough to make the cursor visible.
    ///
    /// Returns whether the scroll position changed.
    pub fn scroll_to_cursor(
        &self,
        mut scroll_controller: ScrollController,
        line_height: f32,
    ) -> bool {
        if self.viewport.width <= 0.
            || self.viewport.height <= 0.
            || line_height <= 0.
            || self.metrics.char_width <= 0.
        {
            return false;
        }

        let (scroll_x, scroll_y) = scroll_controller.into();
        let scrolled_x = -scroll_x as f32;
        let scrolled_y = -scroll_y as f32;
        let cursor_left = self.cursor_col() as f32 * self.metrics.char_width;
        let cursor_right = cursor_left + self.metrics.char_width;
        let cursor_top = self.cursor_row() as f32 * line_height;
        let cursor_bottom = cursor_top + line_height;

        let horizontal_target = if cursor_left < scrolled_x {
            Some(cursor_left)
        } else if cursor_right > scrolled_x + self.viewport.width {
            Some(cursor_right - self.viewport.width)
        } else {
            None
        };

        let vertical_target = if cursor_top < scrolled_y {
            Some(cursor_top)
        } else if cursor_bottom > scrolled_y + self.viewport.height {
            Some(cursor_bottom - self.viewport.height)
        } else {
            None
        };

        let mut changed = false;

        if let Some(target) = horizontal_target {
            changed |= scroll_controller.scroll_to_x(-target as i32);
        }

        if let Some(target) = vertical_target {
            changed |= scroll_controller.scroll_to_y(-target as i32);
        }

        changed
    }

    /// Reconfigures the highlighter with the current language and theme.
    fn configure_highlighter(&mut self) {
        if self.background.is_some() {
            self.invalidate_syntax();
            self.parse();
        } else {
            self.metrics
                .highlighter
                .set_language(self.language.as_ref(), &self.theme);
        }
    }

    /// Sets the language used for syntax highlighting, or disables it with `None`.
    pub fn set_language(&mut self, language: impl Into<Option<EditorLanguage>>) {
        self.language = language.into();
        self.configure_highlighter();
    }

    pub fn is_edited(&self) -> bool {
        self.history.current_change() != self.last_saved_history_change
    }

    pub fn mark_as_saved(&mut self) {
        self.last_saved_history_change = self.history.current_change();
    }

    pub fn measure(&mut self, font_size: f32, font_family: &str) {
        self.metrics
            .measure_longest_line(font_size, font_family, &self.rope);
    }

    pub fn set_theme(&mut self, theme: EditorSyntaxTheme) {
        if self.theme == theme {
            return;
        }
        self.theme = theme;
        self.configure_highlighter();
    }

    pub fn process(
        &mut self,
        font_size: f32,
        font_family: &str,
        edit_event: EditableEvent,
    ) -> bool {
        let mut processed = false;
        match edit_event {
            EditableEvent::Down {
                location,
                editor_line,
                holder,
            } => {
                let holder = holder.0.borrow();
                let ParagraphHolderInner {
                    paragraph,
                    scale_factor,
                    ..
                } = holder.as_ref().unwrap();

                let current_selection = self.selection().clone();

                if self.dragging.shift || self.dragging.clicked {
                    self.selection_mut().set_as_range();
                } else {
                    self.clear_selection();
                }

                if &current_selection != self.selection() {
                    processed = true;
                }

                let char_position = paragraph.get_glyph_position_at_coordinate(
                    location.mul(*scale_factor).to_i32().to_tuple(),
                );
                let press_selection =
                    self.measure_selection(char_position.position as usize, editor_line);

                let press_type = EventsCombos::<()>::pressed(location);
                let new_selection = match press_type {
                    PressEventType::Quadruple => {
                        TextSelection::new_range((0, self.rope.len_utf16_cu()))
                    }
                    PressEventType::Triple => {
                        TextSelection::new_range(self.find_line_boundaries(press_selection.pos()))
                    }
                    PressEventType::Double => {
                        TextSelection::new_range(self.find_word_boundaries(press_selection.pos()))
                    }
                    PressEventType::Single => press_selection,
                };

                self.dragging
                    .start_selection(press_type, new_selection.clone());

                if *self.selection() != new_selection {
                    *self.selection_mut() = new_selection;
                    processed = true;
                }
            }
            EditableEvent::Move {
                location,
                editor_line,
                holder,
            } => {
                if self.dragging.clicked {
                    EventsCombos::<()>::moved(location);

                    let paragraph = holder.0.borrow();
                    let ParagraphHolderInner {
                        paragraph,
                        scale_factor,
                        ..
                    } = paragraph.as_ref().unwrap();

                    let dist_position = location.mul(*scale_factor);

                    // Calculate the end of the highlighting
                    let dist_char = paragraph
                        .get_glyph_position_at_coordinate(dist_position.to_i32().to_tuple());
                    let to = dist_char.position as usize;

                    if self.get_selection().is_none() {
                        self.selection_mut().set_as_range();
                        processed = true;
                    }

                    let current_selection = self.selection().clone();

                    let new_selection = self.dragging.measure_selection(self, to, editor_line);

                    // Update the cursor if it has changed
                    if current_selection != new_selection {
                        *self.selection_mut() = new_selection;
                        processed = true;
                    }
                }
            }
            EditableEvent::Release => {
                self.dragging.clicked = false;
                EventsCombos::<()>::released();
            }
            EditableEvent::KeyDown {
                key,
                modifiers,
                editor_line,
                holder,
            } => {
                match key {
                    // Handle dragging
                    Key::Named(NamedKey::Shift) => {
                        self.dragging.shift = true;
                    }
                    // Handle editing
                    _ => {
                        let event = self.process_key(
                            key,
                            &modifiers,
                            editor_line,
                            holder,
                            true,
                            true,
                            true,
                            true,
                        );
                        if event.contains(TextEvent::TEXT_CHANGED) {
                            self.parse();
                            self.measure(font_size, font_family);
                            self.dragging = TextDragging::default();
                        }
                        if !event.is_empty() {
                            processed = true;
                        }
                    }
                }
            }
            EditableEvent::KeyUp { key, .. } => {
                if *key == Key::Named(NamedKey::Shift) {
                    self.dragging.shift = false;
                }
            }
        };
        processed
    }
}

impl Display for CodeEditorData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.rope.to_string())
    }
}

#[cfg(test)]
mod tests;
