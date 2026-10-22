//! Transient native lookup keeps the mounted manuscript and its input history intact.
mod choices;
mod index;
#[cfg(test)]
mod tests;
pub(super) mod view;
use super::*;
use crate::rules::selection;
use recite_config::WriterCommand;
use recite_writer_session::commands::targets;
use recite_writer_session::location::Location;
pub(crate) use view::{ActiveResult, QueryField};
const PAGE: usize = 5;
/// One opening has a distinct authority even when its mode and query are identical.
#[derive(Clone)]
pub(super) struct PickerInstance(Arc<()>);
impl PartialEq for PickerInstance {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
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
    RulesChoices,
    Destinations,
    RenameBeat,
}
#[derive(Clone, Copy)]
pub(crate) enum Search {
    Commands,
    GoTo,
    ProjectText,
}
enum Purpose {
    Search(Search),
    Editor(Box<crate::choices::Request>),
}
impl Purpose {
    fn mode(&self) -> Mode {
        match self {
            Self::Search(Search::Commands) => Mode::Commands,
            Self::Search(Search::GoTo) => Mode::GoTo,
            Self::Search(Search::ProjectText) => Mode::ProjectText,
            Self::Editor(request) => match request.as_ref() {
                crate::choices::Request::Rules(_) => Mode::RulesChoices,
                crate::choices::Request::Destination(_) => Mode::Destinations,
                crate::choices::Request::Rename(_) => Mode::RenameBeat,
            },
        }
    }
}
#[derive(Clone)]
enum Item {
    Command(WriterCommand),
    Location(Location),
    Lookup(targets::Target),
    Rule(selection::Choice),
    Destination(String),
    RenameBeat(String),
}
struct Entry {
    item: Item,
    title: String,
    detail: String,
}
pub(super) struct Picker {
    purpose: Purpose,
    pub(super) revision: PickerRevision,
    pub(super) instance: PickerInstance,
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
            Self::RulesChoices => "Choose reply rule value",
            Self::Destinations => "Change destination",
            Self::RenameBeat => "Choose a beat to rename",
        }
    }
}
impl Picker {
    fn new(purpose: Purpose, entries: Vec<Entry>) -> Self {
        let matches = (0..entries.len()).collect();
        Self {
            purpose,
            instance: PickerInstance(Arc::new(())),
            revision: PickerRevision::default(),
            query: String::new(),
            total: entries.len(),
            entries,
            matches,
            active: 0,
            limit: 100,
        }
    }
    pub(super) fn mode(&self) -> Mode {
        self.purpose.mode()
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
    pub(super) fn open_picker(&mut self, search: Search) {
        if self.settings_open
            || self.show_welcome
            || self
                .writer
                .as_ref()
                .is_none_or(|writer| writer.phase != Phase::Writing)
        {
            return;
        }
        match index::entries(self, search) {
            Ok(entries) => self.picker = Some(Picker::new(Purpose::Search(search), entries)),
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
    pub(super) fn submit_picker(&mut self) {
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
            Some(Item::Lookup(target)) => {
                let query = self
                    .picker
                    .as_ref()
                    .map(|picker| picker.query.clone())
                    .unwrap_or_default();
                if let Some(writer) = &mut self.writer {
                    writer.lookup(query, target);
                    if writer.navigation.busy() {
                        self.dismiss_picker();
                    }
                }
            }
            Some(Item::Location(location)) => {
                if let Some(writer) = &mut self.writer {
                    writer.navigate(crate::navigation::Target::Location(location));
                    if writer.navigation.busy() {
                        self.dismiss_picker();
                    }
                }
            }
            Some(item @ (Item::Rule(_) | Item::Destination(_) | Item::RenameBeat(_))) => {
                self.activate_editor_choice(item)
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
        let idle = !writer.work.busy() && !writer.navigation.busy() && !writer.session.read_only();
        match command {
            C::Commands | C::GoTo | C::Find | C::OpenProject | C::Settings | C::Close => true,
            C::Save | C::SaveAll => {
                idle && !writer.localisation.saving()
                    && (command != C::SaveAll || !writer.localisation.working())
                    && !writer.source_repair_pending()
                    && writer.session.project().is_some()
            }
            C::Source => idle && !writer.external.busy() && writer.session.project().is_some(),
            C::Apply => idle && writer.session.model().has_draft(),
            C::Undo => {
                idle && (writer.session.model().has_draft()
                    || writer.session.model().document().can_undo()
                    || writer.has_project_history(false))
            }
            C::Redo => {
                idle && (writer.session.model().document().can_redo()
                    || writer.has_project_history(true))
            }
            C::Script | C::Map | C::Split | C::AddBeat | C::Focus => idle,
            C::PaneLeft | C::PaneRight | C::PaneUp | C::PaneDown => true,
            C::NextMatch | C::PreviousMatch => idle,
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
            C::Declarations | C::Build | C::Rename => idle && writer.session.project().is_some(),
            C::Localise => !writer.project_action_blocked() && writer.session.project().is_some(),
            C::Back | C::Forward => {
                idle && writer
                    .navigation
                    .history
                    .step(command == C::Forward)
                    .is_some()
            }
        }
    }
}
