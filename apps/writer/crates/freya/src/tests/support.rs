use crate::editing::{Pane, Writer};
use freya::prelude::*;
use recite_writer_model::{Document, View, Workbench};

pub(crate) const A: &str = "11111111111111111111";
pub(crate) const B: &str = "22222222222222222222";
pub(crate) const C: &str = "33333333333333333333";
pub(crate) const SOURCE: &str = ":: start default\n> a@11111111111111111111\n  Before A.\n> b@22222222222222222222\n  Before B.\n> c@33333333333333333333\n  Before C.\n-> END\n";
pub(crate) fn model() -> Workbench {
    let mut model =
        Workbench::from_document(Document::new(SOURCE).unwrap(), Some(View::Source)).unwrap();
    model.select(View::Passage(A.into())).unwrap();
    model
}
pub(crate) fn writer() -> Writer {
    let preferences = use_state(|| crate::preferences::Preferences::load(None));
    Writer {
        vim: crate::commands::Navigation::new(),
        command_search: crate::commands::Search::new(),
        source_viewport: crate::source_editor::EditorViewport::new(),
        layout: crate::presentation::Layout::new(preferences),
        trial: crate::preview_panel::TrialInputs::new(),
        work: crate::editor_work::EditorWork::new(),
        declaration_buffer: use_state(|| None),
        projections: use_state(|| None),
        localisation: use_state(crate::localisation::Localisation::default),
        files: use_state(|| None),
        examples: use_state(std::collections::BTreeMap::new),
        queue: crate::navigation::Queue {
            search: use_state(String::new),
            attention: use_state(|| false),
            page: use_state(|| 0),
        },
        reference: use_state(|| None),
        buffers: crate::buffers::Buffers {
            bookmarks: crate::buffers::Bookmarks::new(),
            rules: use_state(|| None),
            model: use_state(|| Ok(model())),
            editor: use_state(|| crate::editing::editor_data("Before A.", false, false)),
            prose: use_state(|| "Before A.".into()),
        },
        message: crate::feedback::Feedback::new(),
        dark: false,
        scroll: use_scroll_controller(ScrollConfig::default),
        pane: use_state(|| Pane::Script),
        preferences,
        settings_open: use_state(|| false),
        settings_return_focus: use_state(AccessibilityId::new_unique),
        map_focus: use_a11y(),
        sidebar_focus: use_a11y(),
        inspector_focus: use_a11y(),
        search_focus: use_a11y(),
        selection: use_state(|| Some("start".into())),
        search: use_state(String::new),
        expanded: use_state(std::collections::BTreeSet::new),
    }
}
