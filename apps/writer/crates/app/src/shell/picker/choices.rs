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
            Request::Speaker(request) => request
                .choices
                .iter()
                .map(|choice| Entry {
                    title: choice.title.clone(),
                    detail: choice.detail.clone(),
                    item: Item::Speaker(choice.value.clone()),
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
        let current = request.current().map(str::to_owned);
        writer.choosing = request.opener();
        let mut picker = Picker::new(Purpose::Editor(Box::new(request)), entries);
        // The slip opens on the value the label holds now.
        if let Some(index) = current.and_then(|current| {
            picker.entries.iter().position(|entry| match &entry.item {
                Item::Destination(value) | Item::Speaker(value) => value == &current,
                _ => false,
            })
        }) {
            picker.select(index);
        }
        self.picker = Some(picker);
    }
    /// The choice slip closed, so its label is no longer expanded.
    pub(super) fn forget_choice(&mut self) {
        if let Some(writer) = &mut self.writer {
            writer.choosing = None;
        }
    }
    pub(super) fn activate_editor_choice(&mut self, item: Item) {
        self.forget_choice();
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
            (Request::Speaker(request), Item::Speaker(speaker)) => request.apply(writer, speaker),
            (Request::Rename(request), Item::RenameBeat(beat)) => request.apply(writer, beat),
            _ => writer.status = "Reopen the editor choice to continue.".into(),
        }
    }
}
