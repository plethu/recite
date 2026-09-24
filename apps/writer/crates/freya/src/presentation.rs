//! Live workspace presentation; durable preferences remain in recite-config.
mod sessions;
use crate::{editing::Writer, preferences::Preferences};
use freya::prelude::*;
use recite_config::{UserConfigEdit, WriterPresentation, WriterPresentationField, WriterView};

#[derive(Clone, Copy)]
pub(crate) struct Typography(pub State<Preferences>);

pub(crate) fn typography() -> WriterPresentation {
    try_consume_context::<Typography>()
        .map(|v| v.0.read().config.writer.presentation)
        .unwrap_or_default()
}

#[derive(Clone, Copy)]
pub(crate) struct Layout {
    sessions: sessions::Sessions,
    pub project_controls: State<Option<EventHandler<()>>>,
    pub cameras: State<std::collections::BTreeMap<String, crate::scene_map::camera::Camera>>,
    pub view: State<WriterView>,
    pub focus: State<bool>,
    pub before_focus: State<Option<(crate::editing::Pane, WriterView, AccessibilityId)>>,
    pub drawer_width: State<f32>,
    pub script_width: State<f32>,
    pub navigation: State<bool>,
    pub available: State<f32>,
}
impl Layout {
    pub fn new(preferences: State<Preferences>) -> Self {
        use WriterPresentationField::*;
        Self {
            sessions: sessions::Sessions::new(),
            project_controls: use_state(|| None),
            cameras: use_state(std::collections::BTreeMap::new),
            view: use_state(move || preferences.peek().config.writer.view),
            focus: use_state(|| false),
            before_focus: use_state(|| None),
            drawer_width: use_state(move || {
                f32::from(
                    preferences
                        .peek()
                        .config
                        .writer
                        .presentation
                        .value(DrawerWidth),
                )
            }),
            script_width: use_state(move || {
                f32::from(
                    preferences
                        .peek()
                        .config
                        .writer
                        .presentation
                        .value(ScriptWidth),
                )
            }),
            navigation: use_state(|| true),
            available: use_state(|| 1200.),
        }
    }
    pub fn navigation_expanded(self) -> bool {
        *self.navigation.read()
            && !*self.focus.read()
            && *self.available.read() >= 900. * crate::design::tokens::ui_scale()
    }
    pub fn show_split(self, writer: Writer) -> bool {
        let scale = crate::design::tokens::ui_scale();
        let minimum = 640.
            + if self.navigation_expanded() { 180. } else { 0. }
            + 2. * crate::design::tokens::SPLITTER_WIDTH;
        !*self.focus.read()
            && (writer.preferences.read().config.writer.presentation.split()
                || (!self.standalone() && *writer.pane.read() == crate::editing::Pane::Script))
            && *self.available.read() >= minimum * scale
    }
    pub fn standalone(self) -> bool {
        *self.focus.read() || *self.view.read() == WriterView::Script
    }
}

pub(crate) fn persist(mut writer: Writer, value: WriterPresentation) {
    if let Err(error) = writer
        .preferences
        .write()
        .update(UserConfigEdit::WriterPresentation(value))
    {
        writer.message.error(error);
    }
}
pub(crate) fn resize(writer: Writer, field: WriterPresentationField, width: f32) {
    let value = writer.preferences.peek().config.writer.presentation;
    match value.with_value(field, width.round() as u16) {
        Ok(next) => persist(writer, next),
        Err(error) => {
            let mut writer = writer;
            writer.message.error(error.to_string());
        }
    }
}
