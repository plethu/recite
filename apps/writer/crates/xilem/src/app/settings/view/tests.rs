use super::*;
use crate::tests::support::NoProxy;
use crate::tests::support::mounted::Mounted;
use recite_config::WriterCommand;
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
    ] {
        mounted.activate(caption);
        assert_eq!(mounted.state.status, "Preferences saved.", "{caption}");
    }
    for (field, up) in [
        ("Reading text size", true),
        ("Reading text size", false),
        ("UI scale", true),
        ("UI scale", false),
    ] {
        mounted.step(field, up);
        assert_eq!(mounted.state.status, "Preferences saved.", "{field} {up}");
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

#[test]
fn compact_reading_controls_keep_their_identity_and_fit_before_the_specimen() {
    use recite_config::WriterPresentationField;
    use xilem::masonry::core::WindowEvent;
    let mut mounted = Mounted::new(App::empty(), surface);
    mounted
        .harness
        .process_window_event(WindowEvent::Resize((1280, 720).into()));
    mounted.settle();
    let scale = mounted.find("UI scale");
    for _ in 0..10 {
        mounted.step("UI scale", true);
        assert_eq!(mounted.find("UI scale"), scale);
    }
    assert_eq!(
        mounted
            .state
            .preferences
            .config
            .writer
            .presentation
            .value(WriterPresentationField::UiScale),
        200
    );
    let mut bottom = 0.;
    for name in [
        "Reading text size",
        "Source text size",
        "UI scale",
        "Navigation width",
        "Script width",
    ] {
        let field = mounted.harness.get_widget_with_id(mounted.find(name));
        let field = field
            .ctx()
            .window_transform()
            .transform_rect_bbox(field.ctx().border_box());
        assert!(field.x1 <= 1280. && field.y1 <= 720., "{name}: {field:?}");
        // Each stepper face keeps at least a 24px logical target after doubling the UI scale.
        assert!(
            field.width() >= 2. * 48. && field.height() >= 48.,
            "{field:?}"
        );
        bottom = field.y1;
    }
    let tree = mounted.harness.access_tree();
    let mut pending = vec![tree.state().root()];
    let top = loop {
        let node = pending.pop().expect("writing specimen");
        if node.label().or_else(|| node.value()).as_deref() == Some("Writing preview") {
            break node.bounding_box().unwrap().y0;
        }
        pending.extend(node.children());
    };
    assert!(
        top >= bottom,
        "compact settings put controls before the specimen"
    );
}
