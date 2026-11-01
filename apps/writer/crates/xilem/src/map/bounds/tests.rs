use super::*;
use crate::tests::support::NoProxy;
use xilem::masonry::{core::WindowEvent, kurbo};
use xilem::{ViewCtx, core::View, masonry::testing::TestHarness};

#[test]
fn large_scene_mounts_only_visible_neighbourhood_and_camera_changes_reuse_the_graph() {
    for count in [512, 2048] {
        let source = linear_scene(count);
        let mut fixture = fixture_source(&source).unwrap();
        let writer = &mut fixture.writer;
        let graph = writer.map.scene(writer).unwrap().projection.graph.clone();
        let mut ctx = ViewCtx::new(
            Arc::new(NoProxy),
            Arc::new(tokio::runtime::Runtime::new().unwrap()),
        );
        let mut view = native::surface(writer);
        let (pod, mut local) = view.build(&mut ctx, writer);
        let mut harness = TestHarness::create_with_size(
            crate::presentation::Theme::new(&writer.config.writer).initial_properties(),
            pod.new_widget,
            (1280, 750),
        );
        native::settle(
            writer,
            &mut harness,
            &mut ctx,
            &mut view,
            &mut local,
            native::surface,
        );
        // Preserve the measured canvas workload independently of toolbar reflow.
        let height = writer.map.scene(writer).unwrap().viewport.1;
        harness.process_window_event(WindowEvent::Resize(
            (1280, (750. + 496. - height).round() as u32).into(),
        ));
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
        let mut nodes = 0;
        let mut roles = std::collections::BTreeMap::new();
        let mut names = Vec::new();
        while let Some(node) = pending.pop() {
            nodes += 1;
            *roles.entry(format!("{:?}", node.role())).or_insert(0) += 1;
            if let Some(label) = node.label() {
                names.push(label);
            }
            pending.extend(node.children());
        }
        let scene = writer.map.scene(writer).unwrap();
        // This cost case retains its approximately 500px canvas workload.
        // Larger windows expose more cards and are exercised separately below.
        assert!(
            (490. ..=500.).contains(&scene.viewport.1),
            "{:?}",
            scene.viewport
        );
        assert_eq!(scene.scope.as_ref().unwrap().node_indices.len(), 80);
        assert_eq!(scene.projection.graph.blocks.len(), count);
        // Bound the visible neighbourhood together with its native navigation controls.
        assert!(
            nodes <= 240,
            "bounded Map mounted {nodes} accessibility nodes: {roles:?}; {names:?}"
        );
        assert!(scene.mounted_indices().count() <= 4);
        println!(
            "{count} beats: {nodes} accessible nodes, {} mounted cards",
            scene.mounted_indices().count()
        );
        {
            let scene = writer.map_scene_mut().unwrap();
            scene.whole = true;
            scene.reconcile_selection();
        }
        writer.fit_map();
        native::settle(
            writer,
            &mut harness,
            &mut ctx,
            &mut view,
            &mut local,
            native::surface,
        );
        assert_eq!(
            writer.map.scene(writer).unwrap().mounted_indices().count(),
            0
        );
        let tree = harness.access_tree();
        let mut pending = vec![tree.state().root()];
        let mut overview_nodes = 0;
        while let Some(node) = pending.pop() {
            overview_nodes += 1;
            pending.extend(node.children());
        }
        assert!(
            overview_nodes <= 240,
            "whole-scene overview mounted {overview_nodes} nodes"
        );
        println!(
            "{count} beats in whole-scene Fit: {overview_nodes} accessible nodes, no prose cards"
        );
        let middle = count / 2;
        let point = writer
            .map
            .scene(writer)
            .unwrap()
            .overview_bounds(middle)
            .center();
        writer.select_overview_beat(point);
        assert_eq!(
            writer.map_selected(),
            Some(format!("beat_{middle}").as_str())
        );
        assert_eq!(writer.map.scene(writer).unwrap().detail(), Detail::Dialogue);
        assert!(writer.map.scene(writer).unwrap().mounted_indices().count() <= 4);
        writer.zoom_map(1.2);
        writer.map_input(Source::Viewport, Input::Pan(kurbo::Vec2::new(30., -40.)));
        writer.map_input(Source::Viewport, Input::Begin(Point::ORIGIN));
        for step in 0..64 {
            writer.map_input(
                Source::Viewport,
                Input::Move(Point::new(0., -f64::from(step) * 3_200.)),
            );
            // Four rows can intersect the 160px overscan at 1.2x, plus the
            // selected card retained independently of its visibility.
            assert!(writer.map.scene(writer).unwrap().mounted_indices().count() <= 5);
        }
        writer.cancel_map_gesture();
        assert!(Arc::ptr_eq(
            &graph,
            &writer.map.scene(writer).unwrap().projection.graph
        ));
        assert!(fixture.jobs.try_recv().is_err());
    }
}

#[test]
fn allocated_map_canvas_grows_with_the_window_and_exposes_more_of_the_scene() {
    let mut fixture = fixture_source(&linear_scene(20)).unwrap();
    let writer = &mut fixture.writer;
    let mut ctx = ViewCtx::new(
        Arc::new(NoProxy),
        Arc::new(tokio::runtime::Runtime::new().unwrap()),
    );
    let mut view = native::surface(writer);
    let (pod, mut local) = view.build(&mut ctx, writer);
    let mut harness = TestHarness::create_with_size(
        crate::presentation::Theme::new(&writer.config.writer).initial_properties(),
        pod.new_widget,
        (1280, 750),
    );
    native::settle(
        writer,
        &mut harness,
        &mut ctx,
        &mut view,
        &mut local,
        native::surface,
    );
    let initial = writer.map.scene(writer).unwrap().viewport;
    let initial_cards = writer.map.scene(writer).unwrap().mounted_indices().count();
    harness.process_window_event(WindowEvent::Resize((1280, 1100).into()));
    native::settle(
        writer,
        &mut harness,
        &mut ctx,
        &mut view,
        &mut local,
        native::surface,
    );
    let expanded = writer.map.scene(writer).unwrap().viewport;
    assert_eq!(expanded.0, initial.0);
    assert!(
        (expanded.1 - initial.1 - 350.).abs() < 1.,
        "{initial:?} -> {expanded:?}"
    );
    assert_eq!(
        writer
            .map
            .scene(writer)
            .unwrap()
            .projection
            .graph
            .blocks
            .len(),
        20
    );
    assert!(writer.map.scene(writer).unwrap().mounted_indices().count() > initial_cards);
    assert!(fixture.jobs.try_recv().is_err());
}

pub(super) fn linear_scene(count: usize) -> String {
    let mut source = String::new();
    for index in 0..count {
        source.push_str(&format!(
            ":: beat_{index}{}\n> @line_{index}\n  Dialogue for beat {index}.\n-> {}\n",
            if index == 0 { " default" } else { "" },
            if index + 1 == count {
                "END".into()
            } else {
                format!("beat_{}", index + 1)
            }
        ));
    }
    source
}

#[test]
fn panning_across_mount_boundaries_retains_surviving_native_cards() {
    let source = linear_scene(128);
    let mut fixture = fixture_source(&source).unwrap();
    let writer = &mut fixture.writer;
    writer.map_scene_mut().unwrap().selected = Some("beat_10".into());
    let graph = writer.map.scene(writer).unwrap().projection.graph.clone();
    let positions = writer
        .map
        .scene(writer)
        .unwrap()
        .projection
        .positions
        .clone();
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
    let selected = native::named(&harness, "Beat 10");
    let initially_mounted = writer
        .map
        .scene(writer)
        .unwrap()
        .mounted_indices()
        .collect::<Vec<_>>();
    for _ in 0..32 {
        writer.map_input(Source::Viewport, Input::Pan(kurbo::Vec2::new(0., -32.)));
        native::settle(
            writer,
            &mut harness,
            &mut ctx,
            &mut view,
            &mut local,
            native::surface,
        );
        let scene = writer.map.scene(writer).unwrap();
        assert_eq!(
            native::named(&harness, "Beat 10"),
            selected,
            "a surviving card must not remount when its vector slot changes"
        );
        assert!(
            scene.mounted_indices().count() <= 6,
            "mount work stays bounded by this fixed viewport"
        );
        assert!(Arc::ptr_eq(&graph, &scene.projection.graph));
        assert!(Arc::ptr_eq(&positions, &scene.projection.positions));
    }
    assert_ne!(
        writer
            .map
            .scene(writer)
            .unwrap()
            .mounted_indices()
            .collect::<Vec<_>>(),
        initially_mounted
    );
    assert!(fixture.jobs.try_recv().is_err());
    assert_eq!(writer.session.model().document().source(), source);
    assert!(!writer.session.model().document().can_undo());
}
