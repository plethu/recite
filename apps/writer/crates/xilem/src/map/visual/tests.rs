use super::*;
use crate::tests::support::{NoProxy, mounted::Mounted};
use xilem::masonry::accesskit::{Role, Toggled};
use xilem::masonry::core;
use xilem::masonry::widgets;
use xilem::{ViewCtx, core::View, masonry::testing::TestHarness};

fn manuscript(harness: &TestHarness<widgets::Passthrough>) -> Option<core::WidgetId> {
    let mut pending = vec![harness.root_widget().as_dyn()];
    while let Some(widget) = pending.pop() {
        if let Some(input) = widget.downcast::<widgets::TextInput>() {
            return Some(input.area_pod().id());
        }
        pending.extend(widget.children());
    }
    None
}

#[test]
fn compact_split_keeps_the_manuscript_visible_and_retains_the_requested_workspace() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    writer.viewport.width = 1280.;
    writer.map.workspace = Workspace::Split;
    let mut config = writer.config.clone();
    config.writer.presentation = config
        .writer
        .presentation
        .with_value(recite_config::WriterPresentationField::UiScale, 200)
        .unwrap();
    writer.configure(config);
    let mounted = Mounted::new(fixture.writer, |writer| native::surface(writer));
    assert!(manuscript(&mounted.harness).is_some());
    assert!(!mounted.contains_text("Scene map"));
    // No in-sheet hint: the status line explains, and Script shows as the honoured mode.
    assert!(!mounted.contains_text("Script shown · widen the window for Split"));
    // The Split segment keeps its name and is disabled; the status line says why.
    assert!(mounted.contains_text("Split needs a wider window"));
    assert!(
        mounted
            .harness
            .get_widget_with_id(mounted.find("Split"))
            .ctx()
            .is_disabled()
    );
    let tree = mounted.harness.access_tree();
    let mut pending = vec![tree.state().root()];
    let mut checked = Vec::new();
    while let Some(node) = pending.pop() {
        if node.role() == Role::RadioButton && node.toggled() == Some(Toggled::True) {
            checked.extend(node.label());
        }
        pending.extend(node.children());
    }
    assert!(checked.iter().any(|label| label == "Script"), "{checked:?}");
    assert!(!checked.iter().any(|label| label == "Split"), "{checked:?}");
    assert_eq!(mounted.state.map.workspace, Workspace::Split);
}

#[test]
fn scene_map_renders_at_personal_scales_and_split_workspace() {
    for scale in [100, 200] {
        for workspace in [Workspace::Map, Workspace::Split] {
            let mut fixture = fixture().unwrap();
            let writer = &mut fixture.writer;
            let width = if scale == 200 && workspace == Workspace::Split {
                2400
            } else {
                1440
            };
            writer.viewport.width = f64::from(width);
            writer.map.workspace = workspace;
            let mut config = writer.config.clone();
            config.writer.presentation = config
                .writer
                .presentation
                .with_value(recite_config::WriterPresentationField::UiScale, scale)
                .unwrap();
            writer.configure(config);
            let mut ctx = ViewCtx::new(
                Arc::new(NoProxy),
                Arc::new(tokio::runtime::Runtime::new().unwrap()),
            );
            let mut view = native::surface(writer);
            let (pod, mut local) = view.build(&mut ctx, writer);
            let mut harness = TestHarness::create_with_size(
                crate::presentation::Theme::new(&writer.config.writer).initial_properties(),
                pod.new_widget,
                (width, 1000),
            );
            native::settle(
                writer,
                &mut harness,
                &mut ctx,
                &mut view,
                &mut local,
                native::surface,
            );
            let tree = harness.access_tree();
            let mut pending = vec![tree.state().root()];
            let mut labels = vec![];
            while let Some(node) = pending.pop() {
                if let Some(label) = node.label() {
                    labels.push(label);
                }
                pending.extend(node.children());
            }
            assert!(labels.iter().any(|label| label == "Scene map"));
            if workspace == Workspace::Split {
                assert!(
                    manuscript(&harness).is_some(),
                    "Split retains writing alongside Map at both interface scales"
                );
                assert_eq!(writer.map.workspace, Workspace::Split);
            }
            assert!(
                labels.iter().any(|label| label.contains("Start")),
                "{labels:?}"
            );
            for dpi in [1., 1.25, 1.] {
                harness.process_window_event(core::WindowEvent::Rescale(dpi));
                native::settle(
                    writer,
                    &mut harness,
                    &mut ctx,
                    &mut view,
                    &mut local,
                    native::surface,
                );
                let scene = writer.map.scene(writer).unwrap();
                let card = harness.get_widget_with_id(native::named(&harness, "Start"));
                let bounds = card
                    .ctx()
                    .window_transform()
                    .transform_rect_bbox(card.ctx().border_box());
                assert!(
                    (bounds.width() - f64::from(scene.node(0).width * scene.camera.zoom)).abs()
                        < 0.01,
                    "{scale}% UI, {dpi} DPI: camera zoom must be the sole world-to-logical scale: {bounds:?}"
                );
            }
            if let Ok(prefix) = std::env::var("RECITE_CAPTURE_MAP") {
                harness
                    .render()
                    .save(format!("{prefix}-{workspace:?}-{scale}.png"))
                    .unwrap();
                if workspace == Workspace::Split {
                    let field = manuscript(&harness).expect("Split retains its manuscript");
                    harness.scroll_into_view(field);
                    harness
                        .render()
                        .save(format!("{prefix}-{workspace:?}-manuscript-{scale}.png"))
                        .unwrap();
                }
            }
        }
    }
}
