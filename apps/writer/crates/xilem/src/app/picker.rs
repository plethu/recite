//! Transient native lookup keeps the mounted manuscript and its input history intact.
mod index;
#[cfg(test)]
mod tests;
pub(super) mod view;
use super::*;
use recite_config::WriterCommand;
use recite_writer_session::location::Location;
pub(crate) use view::{ActiveResult, QueryField};
const PAGE: usize = 5;
/// A change to the query, result set or active result; independent of command freshness.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct PickerRevision(u64);
impl PickerRevision {
    fn advance(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    Commands,
    GoTo,
    ProjectText,
}
#[derive(Clone)]
enum Item {
    Command(WriterCommand),
    Location(Location),
}
struct Entry {
    item: Item,
    title: String,
    detail: String,
}
pub(super) struct Picker {
    pub(super) mode: Mode,
    pub(super) revision: PickerRevision,
    query: String,
    entries: Vec<Entry>,
    matches: Vec<usize>,
    active: usize,
    limit: usize,
    total: usize,
}
impl Mode {
    fn title(self) -> &'static str {
        match self {
            Self::Commands => "Commands",
            Self::GoTo => "Go to scene or beat",
            Self::ProjectText => "Find saved project text",
        }
    }
}
impl Picker {
    fn new(mode: Mode, entries: Vec<Entry>) -> Self {
        let matches = (0..entries.len()).collect();
        Self {
            mode,
            revision: PickerRevision::default(),
            query: String::new(),
            total: entries.len(),
            entries,
            matches,
            active: 0,
            limit: 100,
        }
    }
    fn step(&mut self, forward: bool) {
        let count = self.matches.len();
        if count != 0 {
            self.active = (self.active + if forward { 1 } else { count - 1 }) % count;
            self.revision.advance();
        }
    }
}
impl App {
    pub(super) fn open_picker(&mut self, mode: Mode) {
        if self.settings_open
            || self.show_welcome
            || self
                .writer
                .as_ref()
                .is_none_or(|writer| writer.phase != Phase::Writing)
        {
            return;
        }
        match index::entries(self, mode) {
            Ok(entries) => self.picker = Some(Picker::new(mode, entries)),
            Err(error) => self.status = error,
        }
    }
    pub(super) fn dismiss_picker(&mut self) {
        self.picker = None;
    }
    pub(super) fn step_picker(&mut self, forward: bool) {
        if let Some(picker) = &mut self.picker {
            picker.step(forward);
        }
    }
    fn filter_picker(&mut self, query: String) {
        if let Some(picker) = &mut self.picker {
            picker.query = query;
            picker.active = 0;
            picker.limit = 100;
        }
        self.refresh_picker();
    }
    fn submit_picker(&mut self) {
        let active = self.picker.as_ref().map_or(0, |picker| picker.active);
        self.activate_picker(active);
    }
    fn previous_picker_page(&mut self) {
        if let Some(picker) = &mut self.picker {
            picker.active = picker.active.saturating_sub(PAGE);
            picker.revision.advance();
        }
    }
    fn next_picker_page(&mut self) {
        if let Some(picker) = &mut self.picker {
            picker.active =
                ((picker.active / PAGE + 1) * PAGE).min(picker.matches.len().saturating_sub(1));
            picker.revision.advance();
        }
    }
    fn load_picker_matches(&mut self) {
        if let Some(picker) = &mut self.picker {
            picker.limit += 100;
        }
        self.refresh_picker();
    }
    fn refresh_picker(&mut self) {
        let search = self
            .writer
            .as_ref()
            .and_then(|writer| writer.session.project())
            .map(|project| project.files().search_index());
        if let Some(picker) = &mut self.picker {
            picker.refresh(self.preferences.config.ui.keymap, search.as_deref());
        }
    }
    fn activate_picker(&mut self, index: usize) {
        let item = self
            .picker
            .as_ref()
            .and_then(|picker| {
                picker
                    .matches
                    .get(index)
                    .and_then(|index| picker.entries.get(*index))
            })
            .map(|entry| entry.item.clone());
        match item {
            Some(Item::Command(command)) if self.command_enabled(command) => {
                self.dismiss_picker();
                self.run_command(command);
            }
            Some(Item::Location(location)) => {
                if let Some(writer) = &mut self.writer {
                    writer.navigate(crate::navigation::Target::Location(location));
                    if writer.navigation.busy() {
                        self.dismiss_picker();
                    }
                }
            }
            _ => {}
        }
    }
    fn command_enabled(&self, command: WriterCommand) -> bool {
        use WriterCommand as C;
        let Some(writer) = &self.writer else {
            return matches!(command, C::OpenProject | C::Settings | C::Close);
        };
        if writer.phase != Phase::Writing {
            return false;
        }
        let idle = !writer.work.busy() && !writer.navigation.busy();
        match command {
            C::Commands | C::GoTo | C::Find | C::OpenProject | C::Settings | C::Close => true,
            C::Save | C::SaveAll => idle && writer.session.project().is_some(),
            C::Source => idle && !writer.external.busy() && writer.session.project().is_some(),
            C::Apply => idle && writer.session.model().has_draft(),
            C::Undo => {
                idle && (writer.session.model().has_draft()
                    || writer.session.model().document().can_undo())
            }
            C::Redo => idle && writer.session.model().document().can_redo(),
            C::AddBeat => idle,
            C::AddLine | C::AddReply => {
                idle && writer
                    .session
                    .model()
                    .selected_block()
                    .ok()
                    .flatten()
                    .is_some()
            }
            C::Preview => idle,
            C::Back | C::Forward => {
                idle && writer
                    .navigation
                    .history
                    .step(command == C::Forward)
                    .is_some()
            }
            _ => false,
        }
    }
}
