use super::*;
use crate::tests::support::NoProxy;
use crate::tests::support::mounted::Mounted;
use recite_config::{Keymap, WriterPaneSide, WriterTheme};
use std::sync::Arc;
use xilem::masonry::core;
use xilem::masonry::widgets;
use xilem::{ViewCtx, core::View, masonry::testing::TestHarness};

fn surface(app: &mut App) -> impl WidgetView<App, Widget = widgets::Passthrough> + use<> {
    crate::keyed::keyed((), settings(app))
}

#[test]
fn personal_preferences_apply_through_mounted_controls_and_retain_shortcut_drafts() {
    let mut mounted = Mounted::new(App::empty(), surface);
    for caption in [
        "Dark theme",
        "Monochrome colours",
        "Reduce animation",
        "Always show shortcut hints",
        "Zoom around the pointer",
        "Confirm before closing",
        "Vim keymap",
        "Script pane on the right",
        "Larger reading text",
        "Smaller reading text",
        "Increase UI scale",
        "Decrease UI scale",
    ] {
        mounted.activate(caption);
        assert_eq!(mounted.state.status, "Preferences saved.", "{caption}");
    }
    assert_eq!(
        mounted.state.preferences.config.writer.theme,
        WriterTheme::Dark
    );
    assert_eq!(mounted.state.preferences.config.ui.keymap, Keymap::Vim);
    assert_eq!(
        mounted.state.preferences.config.writer.pane_side,
        WriterPaneSide::Right
    );
    assert!(mounted.state.preferences.config.writer.monochrome);
    assert!(mounted.state.preferences.config.writer.reduced_motion);
    mounted.activate("Light theme");
    mounted.activate("Standard keymap");
    mounted.activate("Script pane on the left");
    mounted.activate("Keyboard shortcuts");
    mounted.replace_text("Shortcut binding", "Primary+Alt+S");
    assert_eq!(mounted.state.shortcut_draft(), "Primary+Alt+S");
    mounted.activate("Next commands");
    mounted.activate("Previous commands");
    assert_eq!(mounted.state.shortcut_draft(), "Primary+Alt+S");
    let binding = mounted.find("Shortcut binding");
    mounted.replace_text("Search shortcuts", "source");
    assert_eq!(mounted.find("Shortcut binding"), binding);
    assert_eq!(mounted.state.shortcut_draft(), "Primary+Alt+S");
    assert_eq!(mounted.state.settings.page, 0);
    mounted.replace_text("Search shortcuts", "");
    mounted.activate("Save shortcut");
    assert_eq!(
        mounted
            .state
            .preferences
            .config
            .writer
            .shortcuts
            .binding(WriterCommand::Save),
        "Primary+Alt+S"
    );
    assert!(
        mounted
            .state
            .settings
            .feedback
            .as_ref()
            .unwrap()
            .contains("Shortcut saved")
    );
    mounted.activate("Reset shortcuts");
    assert_eq!(
        mounted
            .state
            .preferences
            .config
            .writer
            .shortcuts
            .binding(WriterCommand::Save),
        "Primary+S"
    );
    assert!(mounted.state.settings.drafts.is_empty());
    mounted.activate("Personal preferences");
    assert_eq!(mounted.state.preferences.config.ui.keymap, Keymap::Standard);
}

#[test]
fn personal_controls_expose_their_native_checkbox_names() {
    let mut app = App::empty();
    let view = settings(&app);
    let mut ctx = ViewCtx::new(
        Arc::new(NoProxy),
        Arc::new(tokio::runtime::Runtime::new().unwrap()),
    );
    let (pod, _) = view.build(&mut ctx, &mut app);
    let harness = TestHarness::create_with_size(
        app.theme.initial_properties(),
        core::NewWidget::new(widgets::SizedBox::new(pod.new_widget)),
        (1200, 650),
    );
    let tree = harness.access_tree();
    let mut pending = vec![tree.state().root()];
    let mut labels = Vec::new();
    while let Some(node) = pending.pop() {
        if let Some(label) = node.label().or_else(|| node.value()) {
            labels.push(label);
        }
        pending.extend(node.children());
    }
    assert!(
        labels.iter().any(|label| label == "Confirm before closing"),
        "{labels:?}"
    );
}
