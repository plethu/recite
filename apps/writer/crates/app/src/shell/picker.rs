//! Transient native lookup keeps the mounted manuscript and its input history intact.
mod choices;
mod index;
mod previews;
use super::*;
pub use crate::rules::selection;
use recite_config::WriterCommand;
use recite_writer_session::commands::targets;
use recite_writer_session::location::Location;
pub const PAGE: usize = 5;
/// One opening has a distinct authority even when its mode and query are identical.
#[derive(Clone, Default)]
pub struct PickerInstance(Arc<()>);
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
pub enum Mode {
    Commands,
    GoTo,
    ProjectText,
    Passages,
    RulesChoices,
    Destinations,
    RenameBeat,
}
pub use super::Search;
pub enum Purpose {
    Search(Search),
    Editor(Box<crate::choices::Request>),
}
impl Purpose {
    pub fn mode(&self) -> Mode {
        match self {
            Self::Search(Search::Commands) => Mode::Commands,
            Self::Search(Search::GoTo) => Mode::GoTo,
            Self::Search(Search::ProjectText) => Mode::ProjectText,
            Self::Search(Search::Passages) => Mode::Passages,
            Self::Editor(request) => match request.as_ref() {
                crate::choices::Request::Rules(_) => Mode::RulesChoices,
                crate::choices::Request::Destination(_) => Mode::Destinations,
                crate::choices::Request::Rename(_) => Mode::RenameBeat,
            },
        }
    }
}
#[derive(Clone)]
pub enum Item {
    Command(WriterCommand),
    Location(Location),
    Passage {
        location: Location,
        passage: recite_writer_model::Passage,
    },
    Lookup(targets::Target),
    Rule(selection::Choice),
    Destination(String),
    RenameBeat(String),
}
pub struct Entry {
    pub item: Item,
    pub title: String,
    pub detail: String,
}
pub struct Picker {
    pub purpose: Purpose,
    pub(super) revision: PickerRevision,
    pub instance: PickerInstance,
    pub query: String,
    pub entries: Vec<Entry>,
    pub matches: Vec<usize>,
    pub active: usize,
    limit: usize,
    pub total: usize,
    pub previews: previews::Previews,
}
impl Mode {
    pub fn title(self) -> &'static str {
        match self {
            Self::Commands => "Commands",
            Self::GoTo => "Go to scene or beat",
            Self::ProjectText => "Find project text",
            Self::Passages => "Find in this beat",
            Self::RulesChoices => "Choose reply rule value",
            Self::Destinations => "Change destination",
            Self::RenameBeat => "Choose a beat to rename",
        }
    }
}
impl Picker {
    pub fn new(purpose: Purpose, entries: Vec<Entry>) -> Self {
        let matches = (0..entries.len()).collect();
        let mut picker = Self {
            purpose,
            instance: PickerInstance::default(),
            revision: PickerRevision::default(),
            query: String::new(),
            total: entries.len(),
            entries,
            matches,
            active: 0,
            limit: 100,
            previews: Default::default(),
        };
        picker.prepare_previews();
        picker
    }
    pub fn mode(&self) -> Mode {
        self.purpose.mode()
    }
    fn step(&mut self, forward: bool) {
        let count = self.matches.len();
        if count != 0 {
            self.select((self.active + if forward { 1 } else { count - 1 }) % count);
        }
    }
    fn select(&mut self, active: usize) {
        self.active = active;
        self.revision.advance();
        self.prepare_previews();
    }
    fn prepare_previews(&mut self) {
        if self.mode() == Mode::Passages {
            self.previews
                .prepare(&self.entries, &self.matches, &self.query, self.active);
        }
    }
}
impl Shell {
    pub fn switch_search(&mut self, search: Search) {
        let Some(picker) = self
            .picker
            .as_ref()
            .filter(|picker| matches!(picker.purpose, Purpose::Search(_)))
        else {
            return;
        };
        let query = picker.query.clone();
        self.open_picker(search);
        self.filter_picker(query);
    }
    pub fn open_picker(&mut self, search: Search) {
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
    pub fn dismiss_picker(&mut self) {
        self.picker = None;
    }
    pub fn step_picker(&mut self, forward: bool) {
        if let Some(picker) = &mut self.picker {
            picker.step(forward);
        }
    }
    pub fn filter_picker(&mut self, query: String) {
        if let Some(picker) = &mut self.picker {
            picker.query = query;
            picker.active = 0;
            picker.limit = 100;
        }
        self.refresh_picker();
    }
    pub fn submit_picker(&mut self) {
        let active = self.picker.as_ref().map_or(0, |picker| picker.active);
        self.activate_picker(active);
    }
    pub fn previous_picker_page(&mut self) {
        if let Some(picker) = &mut self.picker {
            picker.select(picker.active.saturating_sub(PAGE));
        }
    }
    pub fn next_picker_page(&mut self) {
        if let Some(picker) = &mut self.picker {
            picker.select(
                ((picker.active / PAGE + 1) * PAGE).min(picker.matches.len().saturating_sub(1)),
            );
        }
    }
    pub fn load_picker_matches(&mut self) {
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
    pub fn activate_picker(&mut self, index: usize) {
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
            Some(Item::Location(location) | Item::Passage { location, .. }) => {
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
    pub fn command_enabled(&self, command: WriterCommand) -> bool {
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

impl Mode {
    /// What this search does and how its choices are committed.
    pub fn guidance(self) -> &'static str {
        match self {
            Mode::Commands => {
                "Up/Down to choose. Enter runs the command. Esc returns to your document."
            }
            Mode::GoTo => {
                "Find a scene or beat by name. Opening keeps your edits; Save writes them to disk."
            }
            Mode::ProjectText => "Saved text · Save to include edits.",
            Mode::Passages => "This beat, including your text edits. Opening keeps your changes.",
            Mode::RulesChoices => {
                "Enter stages a value. Apply or Save commits it; Esc keeps your current draft."
            }
            Mode::Destinations => {
                "Choose a beat or END. Your current draft is applied; Save writes the change to disk."
            }
            Mode::RenameBeat => {
                "Choose a beat to rename. This updates the proposal; Esc keeps the current proposal."
            }
        }
    }
    /// The keys this search answers to.
    pub fn keyboard_help(self) -> &'static str {
        match self {
            Mode::Commands => "Up/Down: choose · Enter: run · Esc: return",
            Mode::RulesChoices | Mode::Destinations | Mode::RenameBeat => {
                "Up/Down: choose · Enter: choose · Esc: return"
            }
            _ => "Up/Down: choose · Enter: open · Esc: return",
        }
    }
}
/// Result totals and paging for the visible matches.
pub fn result_count(picker: Option<&Picker>, page: usize, count: usize) -> String {
    if count <= PAGE && picker.is_none_or(|picker| picker.total <= count) {
        return format!("{count} {}", if count == 1 { "result" } else { "results" });
    }
    let total = picker
        .filter(|picker| picker.total > count)
        .map_or_else(String::new, |picker| format!(" of {}", picker.total));
    format!(
        "{count}{total} results · page {} of {}",
        page + 1,
        count.div_ceil(PAGE).max(1)
    )
}
/// The message shown when a search has no visible matches.
pub fn empty_message(mode: Mode, picker: Option<&Picker>) -> &'static str {
    if mode == Mode::ProjectText && picker.is_some_and(|picker| picker.query.trim().is_empty()) {
        "Enter words to search the saved project."
    } else {
        "No matching results."
    }
}
