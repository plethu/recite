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
}
#[derive(Clone)]
enum Item {
    Command(WriterCommand),
    Location(Location),
    Rule(crate::rules::selection::Choice),
}
struct Entry {
    item: Item,
    title: String,
    detail: String,
}
pub(super) struct Picker {
    pub(super) mode: Mode,
    pub(super) revision: PickerRevision,
    pub(super) instance: PickerInstance,
    rule_choice: Option<crate::rules::selection::Request>,
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
        }
    }
}
impl Picker {
    fn new(mode: Mode, entries: Vec<Entry>) -> Self {
        let matches = (0..entries.len()).collect();
        Self {
            mode,
            instance: PickerInstance(Arc::new(())),
            rule_choice: None,
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
    pub(super) fn open_rule_choices(&mut self) {
        let Some(request) = self
            .writer
            .as_mut()
            .and_then(|writer| writer.rule_controls.pending_choice.take())
        else {
            return;
        };
        if self.settings_open
            || self.show_welcome
            || !self
                .writer
                .as_ref()
                .is_some_and(|writer| request.is_current(writer))
        {
            return;
        }
        let entries = request
            .choices
            .iter()
            .map(|(title, detail, choice)| Entry {
                title: title.clone(),
                detail: detail.clone(),
                item: Item::Rule(choice.clone()),
            })
            .collect();
        let mut picker = Picker::new(Mode::RulesChoices, entries);
        picker.rule_choice = Some(request);
        self.picker = Some(picker);
    }
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
            Some(Item::Rule(choice)) => {
                let request = self
                    .picker
                    .as_mut()
                    .and_then(|picker| picker.rule_choice.take());
                if let (Some(request), Some(writer)) = (request, &mut self.writer) {
                    request.apply(writer, choice);
                }
                self.dismiss_picker();
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
            C::Save | C::SaveAll => {
                idle && !writer.source_repair_pending() && writer.session.project().is_some()
            }
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
