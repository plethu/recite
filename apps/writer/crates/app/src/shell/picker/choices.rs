//! Editor pickers own the receipt required to interpret their entries.
use super::*;
pub use crate::choices::Request;

impl Shell {
    pub fn open_editor_choices(&mut self) {
        let Some(writer) = &mut self.writer else {
            return;
        };
        let Some(request) = writer.pending_choice.take() else {
            return;
        };
        if self.settings_open || self.show_welcome || !request.is_current(writer) {
            return;
        }
        let entries = match &request {
            Request::Rules(request) => request
                .choices
                .iter()
                .map(|(title, detail, choice)| Entry {
                    title: title.clone(),
                    detail: detail.clone(),
                    item: Item::Rule(choice.clone()),
                })
                .collect(),
            Request::Destination(request) => request
                .choices
                .iter()
                .map(|choice| Entry {
                    title: choice.title.clone(),
                    detail: choice.detail.clone(),
                    item: Item::Destination(choice.value.clone()),
                })
                .collect(),
            Request::Rename(request) => request
                .choices
                .iter()
                .map(|choice| Entry {
                    title: choice.title.clone(),
                    detail: choice.detail.clone(),
                    item: Item::RenameBeat(choice.value.clone()),
                })
                .collect(),
        };
        self.picker = Some(Picker::new(Purpose::Editor(Box::new(request)), entries));
    }
    pub(super) fn activate_editor_choice(&mut self, item: Item) {
        let Some(picker) = self.picker.take() else {
            return;
        };
        let Some(writer) = &mut self.writer else {
            return;
        };
        let Purpose::Editor(request) = picker.purpose else {
            writer.status = "Reopen the editor choice to continue.".into();
            return;
        };
        match (*request, item) {
            (Request::Rules(request), Item::Rule(choice)) => {
                request.apply(writer, choice);
            }
            (Request::Destination(request), Item::Destination(destination)) => {
                request.apply(writer, destination);
            }
            (Request::Rename(request), Item::RenameBeat(beat)) => request.apply(writer, beat),
            _ => writer.status = "Reopen the editor choice to continue.".into(),
        }
    }
}
