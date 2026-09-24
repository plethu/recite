//! Trial language selection reuses catalogue setup's cached index and shared picker.
use super::setup::languages;
use crate::{
    design::SearchPicker,
    messages::{MsgId, text},
};
use freya::prelude::*;

#[derive(Clone, PartialEq)]
pub(crate) struct LanguagePicker {
    pub locale: State<String>,
    pub vim: bool,
}
impl Component for LanguagePicker {
    fn render(&self) -> impl IntoElement {
        let mut locale = self.locale;
        let open = use_state(|| false);
        let query = use_state(String::new);
        let choices = use_memo(move || {
            std::sync::Arc::new(if *open.read() {
                languages::matches(&query.read())
            } else {
                Vec::new()
            })
        });
        SearchPicker {
            id: use_a11y(),
            input_id: use_a11y(),
            name: text(MsgId::WriterChooseLanguage),
            placeholder: text(MsgId::WriterLanguageExample),
            empty_hint: text(MsgId::WriterLanguageExample),
            no_matches: text(MsgId::WriterNoLanguages),
            selected: languages::caption(&locale.read()).unwrap_or_default(),
            query,
            open,
            options: choices.read().clone(),
            enabled: true,
            vim: self.vim,
            choose: EventHandler::new(move |option: crate::design::PickerOption| {
                locale.set(option.value)
            }),
        }
    }
}
