//! Window movement never retires the admitted draft before checked navigation accepts it.
use super::{Content, Direction};
use crate::{Phase, Writer, navigation::Target};
#[cfg(test)]
mod tests;

impl Writer {
    pub(crate) fn shift_reading(&mut self, direction: Direction) {
        if self.phase != Phase::Writing || self.navigation.busy() || self.work.busy() {
            return;
        }
        let Some(window) = self
            .reading
            .window
            .shifted(direction, self.reading.rows.len())
        else {
            return;
        };
        let mut range = window.range(self.reading.rows.len());
        if self
            .reading
            .selected_row
            .is_none_or(|row| range.contains(&row))
        {
            self.reading.window = window;
            return;
        }
        let passage = |index: usize| match &self.reading.rows[index].content {
            Content::Passage { passage, .. } => Some(passage.id.clone()),
            _ => None,
        };
        let mut location = self.current_location();
        location.passage = match direction {
            Direction::Next => range.find_map(passage),
            Direction::Previous => range.rev().find_map(passage),
        };
        if let Err(error) = self.begin_navigation(Target::Reading { location, window }) {
            self.status = error;
            self.request_editing();
        }
    }
}
