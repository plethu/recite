use super::*;
use crate::tests::support::NoProxy;
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
fn scene_map_renders_at_personal_scales_and_split_workspace() {
    for scale in [100, 200] {
        for workspace in [Workspace::Map, Workspace::Split] {
            let mut fixture = fixture().unwrap();
            let writer = &mut fixture.writer;
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
                (1440, 1000),
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
            assert!(
                labels
                    .iter()
                    .any(|label| label.contains("Selected · Start")),
                "{labels:?}"
            );
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
