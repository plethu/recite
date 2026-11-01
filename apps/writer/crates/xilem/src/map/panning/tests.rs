//! Drag panning cost follows mount boundaries, not pointer events.
use super::*;
use crate::tests::support::NoProxy;
use recite_writer_session::map::topology::OVERSCAN;
use xilem::{
    ViewCtx, WidgetView,
    core::{DynMessage, MessageCtx, ViewPathTracker},
    masonry::{core::WidgetId, kurbo::Vec2, testing::TestHarness, widgets::Passthrough},
    view::PointerButton,
};

const STEPS: usize = 180;
const STEP: f64 = 6.;

#[derive(Default)]
struct PanCost {
    rebuilds: usize,
    nodes: usize,
    peak_nodes: usize,
    /// Peak accessibility nodes on frames that only translated retained content.
    peak_local_nodes: usize,
}

struct Probe<V: WidgetView<Writer, Widget = Passthrough>> {
    fixture: Fixture,
    harness: TestHarness<Passthrough>,
    ctx: ViewCtx,
    view: V,
    local: V::ViewState,
    make: fn(&Writer) -> V,
}

fn probe<V: WidgetView<Writer, Widget = Passthrough>>(make: fn(&Writer) -> V) -> Probe<V> {
    let mut fixture = fixture_source(&bounds::linear_scene(128)).unwrap();
    let writer = &mut fixture.writer;
    let mut ctx = ViewCtx::new(
        Arc::new(NoProxy),
        Arc::new(tokio::runtime::Runtime::new().unwrap()),
    );
    let mut view = make(writer);
    let (pod, mut local) = view.build(&mut ctx, writer);
    let mut harness = TestHarness::create_with_size(
        crate::presentation::Theme::new(&writer.config.writer).initial_properties(),
        pod.new_widget,
        (1280, 1000),
    );
    native::settle(writer, &mut harness, &mut ctx, &mut view, &mut local, make);
    Probe {
        fixture,
        harness,
        ctx,
        view,
        local,
        make,
    }
}

impl<V: WidgetView<Writer, Widget = Passthrough>> Probe<V> {
    fn writer(&self) -> &Writer {
        &self.fixture.writer
    }
    /// Mirrors the native driver: actions are delivered and the app view is rebuilt
    /// only when the event produced one, then the frame is redrawn.
    fn frame(&mut self, cost: &mut PanCost) {
        let writer = &mut self.fixture.writer;
        let mut acted = false;
        while let Some((action, source)) = self.harness.pop_action_erased() {
            let path = self
                .ctx
                .get_id_path(source)
                .expect("native action has a live view path")
                .clone();
            let mut message = MessageCtx::new(
                std::mem::take(self.ctx.environment()),
                path,
                DynMessage(action),
            );
            self.harness.edit_root_widget(|root| {
                self.view
                    .message(&mut self.local, &mut message, root, writer);
            });
            *self.ctx.environment() = message.finish().0;
            acted = true;
        }
        if acted {
            let next = (self.make)(writer);
            self.harness.edit_root_widget(|root| {
                next.rebuild(&self.view, &mut self.local, &mut self.ctx, root, writer)
            });
            self.view = next;
            cost.rebuilds += 1;
        }
        let (_, update) = self.harness.redraw();
        cost.nodes += update.nodes.len();
        cost.peak_nodes = cost.peak_nodes.max(update.nodes.len());
        if !acted {
            cost.peak_local_nodes = cost.peak_local_nodes.max(update.nodes.len());
        }
    }
    fn viewport(&self) -> WidgetId {
        let tree = self.harness.access_tree();
        let mut pending = vec![tree.state().root()];
        let raw = loop {
            let node = pending.pop().expect("mounted scene map");
            if node.label().as_deref() == Some("Scene map") {
                break node.locate().0.0;
            }
            pending.extend(node.children());
        };
        let mut pending = vec![self.harness.root_widget().as_dyn()];
        while let Some(widget) = pending.pop() {
            if widget.id().to_raw() == raw {
                return widget.id();
            }
            pending.extend(widget.children());
        }
        panic!("Scene map has no native widget");
    }
    /// Empty canvas beside the linear column of cards.
    fn empty_canvas(&self) -> Point {
        let viewport = self.harness.get_widget_with_id(self.viewport());
        let bounds = viewport
            .ctx()
            .window_transform()
            .transform_rect_bbox(viewport.ctx().border_box());
        Point::new(bounds.x0 + 24., bounds.center().y)
    }
    fn camera_y(&self) -> f32 {
        self.writer().map.scene(self.writer()).unwrap().camera.y
    }
    fn card_y(&self, name: &str) -> f64 {
        let card = self
            .harness
            .get_widget_with_id(native::named(&self.harness, name));
        card.ctx().window_transform().translation().y
    }
    fn drag(&mut self) -> PanCost {
        let start = self.empty_canvas();
        let mut cost = PanCost::default();
        self.harness.mouse_move(start);
        self.harness
            .mouse_button_press(Some(PointerButton::Primary));
        self.frame(&mut cost);
        assert!(matches!(
            self.writer().map.scene(self.writer()).unwrap().gesture,
            Gesture::Panning { .. }
        ));
        for step in 1..=STEPS {
            self.harness
                .mouse_move(start - Vec2::new(0., STEP * step as f64));
            self.frame(&mut cost);
        }
        self.harness
            .mouse_button_release(Some(PointerButton::Primary));
        self.frame(&mut cost);
        cost
    }
}

#[test]
fn drag_panning_commits_at_mount_boundaries_and_reexposes_only_the_moved_layer() {
    let mut probe = probe(native::surface);
    let before = probe.writer().map.scene(probe.writer()).unwrap().camera;
    let cost = probe.drag();
    let after = probe.writer().map.scene(probe.writer()).unwrap().camera;
    println!(
        "{STEPS} drag moves: {} rebuilds, {} accessibility nodes ({} peak/frame, {} peak/local frame)",
        cost.rebuilds, cost.nodes, cost.peak_nodes, cost.peak_local_nodes
    );
    // Begin, release and one commit per overscan crossing; local frames carry only the
    // window root and the translated content. Measured: 8 rebuilds and 564 nodes.
    let distance = STEP * STEPS as f64;
    let crossings = (distance / f64::from(OVERSCAN)).ceil();
    assert!(
        cost.rebuilds <= 2 + crossings as usize,
        "{} rebuilds",
        cost.rebuilds
    );
    assert!(
        cost.peak_local_nodes <= 2,
        "{} nodes",
        cost.peak_local_nodes
    );
    assert!(cost.nodes <= 600, "{} nodes", cost.nodes);
    assert_eq!(after.x, before.x);
    assert!((after.y - (before.y - distance as f32)).abs() < 0.5);
    assert!(matches!(
        probe.writer().map.scene(probe.writer()).unwrap().gesture,
        Gesture::Idle
    ));
}

#[test]
fn wheel_scrolling_translates_retained_cards_until_it_settles_or_reaches_the_overscan() {
    let mut probe = probe(native::surface);
    let mut cost = PanCost::default();
    probe.harness.mouse_move(probe.empty_canvas());
    let (camera, card) = (probe.camera_y(), probe.card_y("Beat 1"));
    for _ in 0..3 {
        probe.harness.mouse_wheel(Vec2::new(0., -40.));
        probe.frame(&mut cost);
    }
    assert_eq!(cost.rebuilds, 0);
    assert_eq!(probe.camera_y(), camera);
    assert_eq!(probe.card_y("Beat 1"), card - 120.);
    probe.harness.animate_ms(60);
    probe.frame(&mut cost);
    assert_eq!(
        cost.rebuilds, 0,
        "wheel movement commits only after it settles"
    );
    probe.harness.animate_ms(80);
    probe.frame(&mut cost);
    assert_eq!(cost.rebuilds, 1);
    assert_eq!(probe.camera_y(), camera - 120.);
    assert_eq!(
        probe.card_y("Beat 1"),
        card - 120.,
        "committing must not move cards"
    );
    probe.harness.mouse_wheel(Vec2::new(0., -200.));
    probe.frame(&mut cost);
    assert_eq!(
        cost.rebuilds, 2,
        "a delta past the overscan commits at once"
    );
    assert_eq!(probe.camera_y(), camera - 320.);
}

#[test]
fn pressing_commits_pending_wheel_movement_before_the_drag_begins() {
    let mut probe = probe(native::surface);
    let mut cost = PanCost::default();
    let start = probe.empty_canvas();
    probe.harness.mouse_move(start);
    let camera = probe.camera_y();
    probe.harness.mouse_wheel(Vec2::new(0., -50.));
    probe.frame(&mut cost);
    probe
        .harness
        .mouse_button_press(Some(PointerButton::Primary));
    probe.frame(&mut cost);
    assert_eq!(probe.camera_y(), camera - 50.);
    probe.harness.mouse_move(start - Vec2::new(0., 30.));
    probe
        .harness
        .mouse_button_release(Some(PointerButton::Primary));
    probe.frame(&mut cost);
    assert_eq!(probe.camera_y(), camera - 80.);
}

#[test]
fn escape_during_a_drag_restores_the_gesture_origin_after_intermediate_commits() {
    use xilem::masonry::core::{
        TextEvent,
        keyboard::{Key, KeyState, KeyboardEvent, NamedKey},
    };
    let mut probe = probe(native::surface);
    let mut cost = PanCost::default();
    let start = probe.empty_canvas();
    let (camera, card) = (probe.camera_y(), probe.card_y("Beat 1"));
    probe.harness.mouse_move(start);
    probe
        .harness
        .mouse_button_press(Some(PointerButton::Primary));
    probe.frame(&mut cost);
    for offset in [100., 200., 260.] {
        probe.harness.mouse_move(start - Vec2::new(0., offset));
        probe.frame(&mut cost);
    }
    assert_eq!(
        probe.camera_y(),
        camera - 200.,
        "the second move crossed the overscan"
    );
    assert_eq!(probe.card_y("Beat 1"), card - 260.);
    probe
        .harness
        .process_text_event(TextEvent::Keyboard(KeyboardEvent {
            state: KeyState::Down,
            key: Key::Named(NamedKey::Escape),
            ..KeyboardEvent::default()
        }));
    probe.frame(&mut cost);
    assert_eq!(probe.camera_y(), camera);
    assert_eq!(probe.card_y("Beat 1"), card);
    assert!(matches!(
        probe.writer().map.scene(probe.writer()).unwrap().gesture,
        Gesture::Idle
    ));
}
