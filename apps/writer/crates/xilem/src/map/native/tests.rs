use super::*;
use crate::tests::support::NoProxy;
use xilem::masonry::core;
use xilem::{
    ViewCtx, WidgetView,
    core::{DynMessage, MessageCtx, View as _, ViewPathTracker},
    masonry::{
        accesskit::Role,
        core::{
            TextEvent, WidgetId,
            keyboard::{Key, KeyState, KeyboardEvent, NamedKey},
        },
        testing::TestHarness,
        widgets::Passthrough,
    },
};

pub(super) fn surface(writer: &Writer) -> impl WidgetView<Writer, Widget = Passthrough> + use<> {
    crate::keyed::keyed((), crate::view::writer(writer))
}
pub(super) fn named(harness: &TestHarness<Passthrough>, text: &str) -> WidgetId {
    let tree = harness.access_tree();
    let mut pending = vec![tree.state().root()];
    let mut raw = None;
    while let Some(node) = pending.pop() {
        if node.role() == Role::Button && node.label().is_some_and(|label| label.starts_with(text))
        {
            raw = Some(node.locate().0.0);
            break;
        }
        pending.extend(node.children());
    }
    let mut pending = vec![harness.root_widget().as_dyn()];
    while let Some(widget) = pending.pop() {
        if raw == Some(widget.id().to_raw()) {
            return widget.id();
        }
        pending.extend(widget.children());
    }
    panic!("Missing button {text}");
}
pub(super) fn settle<V: WidgetView<Writer, Widget = Passthrough>>(
    writer: &mut Writer,
    harness: &mut TestHarness<Passthrough>,
    ctx: &mut ViewCtx,
    view: &mut V,
    local: &mut V::ViewState,
    make: fn(&Writer) -> V,
) {
    for _ in 0..20 {
        let mut changed = false;
        while let Some((action, source)) = harness.pop_action_erased() {
            let path = ctx
                .get_id_path(source)
                .expect("native action has a live view path")
                .clone();
            let mut message =
                MessageCtx::new(std::mem::take(ctx.environment()), path, DynMessage(action));
            harness.edit_root_widget(|root| {
                view.message(local, &mut message, root, writer);
            });
            *ctx.environment() = message.finish().0;
            changed = true;
        }
        let next = make(writer);
        harness.edit_root_widget(|root| next.rebuild(view, local, ctx, root, writer));
        *view = next;
        harness.redraw();
        if !changed {
            return;
        }
    }
    panic!("Map native actions did not settle");
}
fn key(harness: &mut TestHarness<Passthrough>, key: NamedKey) {
    for state in [KeyState::Down, KeyState::Up] {
        harness.process_text_event(TextEvent::Keyboard(KeyboardEvent {
            state,
            key: Key::Named(key),
            ..KeyboardEvent::default()
        }));
    }
}

#[test]
fn arrows_focus_reveal_the_whole_destination_card_and_enter_opens_that_beat() {
    for scale in [100, 200] {
        focus_destination(scale);
    }
}
fn focus_destination(scale: u16) {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    let mut config = writer.config.clone();
    config.writer.presentation = config
        .writer
        .presentation
        .with_value(recite_config::WriterPresentationField::UiScale, scale)
        .unwrap();
    writer.configure(config);
    writer.zoom_map(1.2);
    let mut ctx = ViewCtx::new(
        Arc::new(NoProxy),
        Arc::new(tokio::runtime::Runtime::new().unwrap()),
    );
    let mut view = surface(writer);
    let sender = writer.navigation.sender.clone();
    let (pod, mut local) = view.build(&mut ctx, writer);
    writer.navigation.sender = sender;
    let mut harness = TestHarness::create_with_size(
        crate::presentation::Theme::new(&writer.config.writer).initial_properties(),
        pod.new_widget,
        (1280, 720),
    );
    let first = named(&harness, "Selected · Start");
    harness.focus_on(Some(first));
    settle(
        writer,
        &mut harness,
        &mut ctx,
        &mut view,
        &mut local,
        surface,
    );
    key(&mut harness, NamedKey::ArrowDown);
    settle(
        writer,
        &mut harness,
        &mut ctx,
        &mut view,
        &mut local,
        surface,
    );
    assert_eq!(writer.map_selected(), Some("later"));
    assert_eq!(
        harness.focused_widget_id(),
        Some(named(&harness, "Selected · Later"))
    );
    assert!(writer.map.scene(writer).unwrap().focus.is_none());
    for title in ["Selected · Later", "Move Later"] {
        let widget = harness.get_widget_with_id(named(&harness, title));
        let bounds = widget
            .ctx()
            .window_transform()
            .transform_rect_bbox(widget.ctx().border_box());
        assert!(
            bounds.y0 >= -0.01 && bounds.y1 <= 720.01,
            "{title} at scale {scale}: {bounds:?}"
        );
    }
    key(&mut harness, NamedKey::Enter);
    settle(
        writer,
        &mut harness,
        &mut ctx,
        &mut view,
        &mut local,
        surface,
    );
    support::finish_navigation(writer, &mut fixture.jobs).unwrap();
    assert_eq!(
        writer.session.model().selected_block().unwrap().as_deref(),
        Some("later")
    );
    assert_eq!(writer.map.workspace, Workspace::Split);
}

#[test]
fn pointer_capture_survives_a_card_leaving_the_viewport_and_returning() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    let mut ctx = ViewCtx::new(
        Arc::new(NoProxy),
        Arc::new(tokio::runtime::Runtime::new().unwrap()),
    );
    let mut view = surface(writer);
    let sender = writer.navigation.sender.clone();
    let (pod, mut local) = view.build(&mut ctx, writer);
    writer.navigation.sender = sender;
    let mut harness = TestHarness::create_with_size(
        crate::presentation::Theme::new(&writer.config.writer).initial_properties(),
        pod.new_widget,
        (1280, 1000),
    );
    let grip = named(&harness, "Move Start");
    let widget = harness.get_widget_with_id(grip);
    let bounds = widget
        .ctx()
        .window_transform()
        .transform_rect_bbox(widget.ctx().border_box());
    harness.mouse_move(bounds.center());
    harness.mouse_button_press(Some(xilem::view::PointerButton::Primary));
    settle(
        writer,
        &mut harness,
        &mut ctx,
        &mut view,
        &mut local,
        surface,
    );
    assert_eq!(harness.pointer_capture_target_id(), Some(grip));
    harness.mouse_move(Point::new(4000., 4000.));
    settle(
        writer,
        &mut harness,
        &mut ctx,
        &mut view,
        &mut local,
        surface,
    );
    assert_eq!(harness.pointer_capture_target_id(), Some(grip));
    harness.mouse_move(Point::new(600., 400.));
    settle(
        writer,
        &mut harness,
        &mut ctx,
        &mut view,
        &mut local,
        surface,
    );
    assert_eq!(harness.pointer_capture_target_id(), Some(grip));
    key(&mut harness, NamedKey::Escape);
    settle(
        writer,
        &mut harness,
        &mut ctx,
        &mut view,
        &mut local,
        surface,
    );
    assert_eq!(harness.pointer_capture_target_id(), None);
    assert!(matches!(
        writer.map.scene(writer).unwrap().gesture,
        Gesture::Idle
    ));
    assert!(fixture.jobs.try_recv().is_err());
}

#[test]
fn only_a_successful_primary_double_click_opens_the_clicked_card() {
    use xilem::masonry::{
        core::{PointerButtonEvent, PointerEvent, PointerState},
        dpi::PhysicalPosition,
    };
    use xilem::view::PointerButton;
    for (button, count, inside, opens) in [
        (PointerButton::Primary, 1, true, false),
        (PointerButton::Primary, 2, true, true),
        (PointerButton::Secondary, 2, true, false),
        (PointerButton::Primary, 2, false, false),
    ] {
        let mut fixture = fixture().unwrap();
        let writer = &mut fixture.writer;
        let mut ctx = ViewCtx::new(
            Arc::new(NoProxy),
            Arc::new(tokio::runtime::Runtime::new().unwrap()),
        );
        let mut view = surface(writer);
        let sender = writer.navigation.sender.clone();
        let (pod, mut local) = view.build(&mut ctx, writer);
        writer.navigation.sender = sender;
        let mut harness = TestHarness::create_with_size(
            crate::presentation::Theme::new(&writer.config.writer).initial_properties(),
            pod.new_widget,
            (1280, 1000),
        );
        settle(
            writer,
            &mut harness,
            &mut ctx,
            &mut view,
            &mut local,
            surface,
        );
        let id = named(&harness, "Later · End");
        let widget = harness.get_widget_with_id(id);
        let bounds = widget
            .ctx()
            .window_transform()
            .transform_rect_bbox(widget.ctx().border_box());
        let point = bounds.center();
        for down in [true, false] {
            let position = if down || inside {
                point
            } else {
                Point::new(4000., 4000.)
            };
            harness.mouse_move(position);
            let event = PointerButtonEvent {
                pointer: core::PointerInfo {
                    pointer_id: Some(core::PointerId::PRIMARY),
                    pointer_type: core::PointerType::Mouse,
                    persistent_device_id: None,
                },
                button: Some(button),
                state: PointerState {
                    position: PhysicalPosition::new(position.x, position.y),
                    count,
                    ..PointerState::default()
                },
            };
            harness.process_pointer_event(if down {
                PointerEvent::Down(event)
            } else {
                PointerEvent::Up(event)
            });
        }
        settle(
            writer,
            &mut harness,
            &mut ctx,
            &mut view,
            &mut local,
            surface,
        );
        assert_eq!(
            writer.map.workspace == Workspace::Split,
            opens,
            "{button:?}, count={count}, inside={inside}"
        );
        if opens {
            support::finish_navigation(writer, &mut fixture.jobs).unwrap();
            assert_eq!(
                writer.session.model().selected_block().unwrap().as_deref(),
                Some("later")
            );
        } else {
            assert!(fixture.jobs.try_recv().is_err());
        }
    }
}
