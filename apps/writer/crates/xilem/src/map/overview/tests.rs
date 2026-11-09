#![allow(unused_imports)]
use super::super::*;
use super::*;
use crate::tests::support::{NoProxy, native_label};
use kurbo::Rect;
use recite_writer_session::map::layout::HEIGHT;
use xilem::{
    ViewCtx,
    core::View,
    masonry::{
        core::{
            TextEvent, WidgetId,
            keyboard::{Key, NamedKey},
        },
        testing::TestHarness,
        widgets::Passthrough,
    },
};

fn viewport(harness: &TestHarness<Passthrough>) -> WidgetId {
    let mut pending = vec![harness.root_widget().as_dyn()];
    while let Some(widget) = pending.pop() {
        if native_label(harness, widget.id()).as_deref() == Some("Scene map") {
            return widget.id();
        }
        pending.extend(widget.children());
    }
    panic!("Map viewport");
}
#[test]
fn overview_collapse_transfers_card_focus_and_arrows_promote_back_to_a_native_card() {
    for scale in [100, 200] {
        overview_interaction(scale);
    }
}
fn overview_interaction(scale: u16) {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
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
        (1280, 1000),
    );
    native::settle(
        writer,
        &mut harness,
        &mut ctx,
        &mut view,
        &mut local,
        native::surface,
    );
    let canvas = viewport(&harness);
    let start = native::named(&harness, "Start");
    harness.focus_on(Some(start));
    writer.zoom_map(0.4);
    native::settle(
        writer,
        &mut harness,
        &mut ctx,
        &mut view,
        &mut local,
        native::surface,
    );
    assert_eq!(writer.map.scene(writer).unwrap().detail(), Detail::Overview);
    assert_eq!(harness.focused_widget_id(), Some(canvas));
    assert_eq!(writer.map_selected(), Some("start"));
    if let Ok(prefix) = std::env::var("RECITE_CAPTURE_OVERVIEW") {
        harness
            .render()
            .save(format!("{prefix}-{scale}.png"))
            .unwrap();
    }
    harness.process_text_event(TextEvent::key_down(Key::Named(NamedKey::ArrowDown)));
    native::settle(
        writer,
        &mut harness,
        &mut ctx,
        &mut view,
        &mut local,
        native::surface,
    );
    assert_eq!(writer.map.scene(writer).unwrap().detail(), Detail::Dialogue);
    assert_eq!(
        harness.focused_widget_id(),
        Some(native::named(&harness, "Later"))
    );
    let fit = native::named(&harness, "Fit");
    harness.focus_on(Some(fit));
    writer.zoom_map(0.4);
    native::settle(
        writer,
        &mut harness,
        &mut ctx,
        &mut view,
        &mut local,
        native::surface,
    );
    assert_eq!(harness.focused_widget_id(), Some(fit));
    let selected_point = |writer: &Writer, harness: &TestHarness<Passthrough>| {
        let scene = writer.map.scene(writer).unwrap();
        let node = &scene.projection.graph.nodes[1];
        let local = Point::new(
            f64::from((node.x + node.width / 2.) * scene.camera.zoom + scene.camera.x),
            f64::from((node.y + 80.) * scene.camera.zoom + scene.camera.y),
        );
        harness.get_widget_with_id(canvas).ctx().window_transform() * local
    };
    let point = selected_point(writer, &harness);
    harness.mouse_move(point);
    harness.mouse_button_press(Some(xilem::view::PointerButton::Primary));
    harness.mouse_move(point + (30., 0.));
    harness.mouse_button_release(Some(xilem::view::PointerButton::Primary));
    native::settle(
        writer,
        &mut harness,
        &mut ctx,
        &mut view,
        &mut local,
        native::surface,
    );
    assert_eq!(writer.map.scene(writer).unwrap().detail(), Detail::Overview);
    let point = selected_point(writer, &harness);
    harness.mouse_move(point);
    harness.mouse_button_press(Some(xilem::view::PointerButton::Primary));
    harness.mouse_button_release(Some(xilem::view::PointerButton::Primary));
    native::settle(
        writer,
        &mut harness,
        &mut ctx,
        &mut view,
        &mut local,
        native::surface,
    );
    assert_eq!(writer.map.scene(writer).unwrap().detail(), Detail::Dialogue);
    assert_eq!(
        harness.focused_widget_id(),
        Some(native::named(&harness, "Later"))
    );
    assert!(fixture.jobs.try_recv().is_err());
    assert_eq!(writer.session.model().document().source(), SOURCE);
}
