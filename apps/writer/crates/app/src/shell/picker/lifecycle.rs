//! The shell's one picker from opening to submission: switching search, filtering, stepping,
//! scrolling previews into view and activating a result.
use super::*;

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
            Ok(entries) => {
                self.forget_choice();
                self.picker = Some(Picker::new(Purpose::Search(search), entries));
            }
            Err(error) => self.status = error,
        }
    }
    pub fn dismiss_picker(&mut self) {
        self.forget_choice();
        self.picker = None;
    }
    pub fn step_picker(&mut self, forward: bool) {
        if let Some(picker) = &mut self.picker {
            picker.step(forward);
        }
    }
    /// The list scrolled to show `visible`.
    pub fn picker_scrolled(&mut self, visible: Range<usize>) {
        if let Some(picker) = &mut self.picker {
            picker.visible = visible;
        }
    }
    pub fn filter_picker(&mut self, query: String) {
        if let Some(picker) = &mut self.picker {
            picker.query = query;
            picker.active = 0;
            picker.anchor = 0;
            picker.limit = 100;
        }
        self.refresh_picker();
    }
    pub fn submit_picker(&mut self) {
        let active = self.picker.as_ref().map_or(0, |picker| picker.active);
        self.activate_picker(active);
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
            Some(
                item @ (Item::Rule(_)
                | Item::Destination(_)
                | Item::Speaker(_)
                | Item::RenameBeat(_)),
            ) => self.activate_editor_choice(item),
            _ => {}
        }
    }
}
