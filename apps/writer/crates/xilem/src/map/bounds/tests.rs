use super::*;
use crate::tests::support::NoProxy;
use xilem::masonry::kurbo;
use xilem::{ViewCtx, core::View, masonry::testing::TestHarness};

#[test]
fn large_scene_mounts_only_visible_neighbourhood_and_camera_changes_reuse_the_graph() {
    for count in [512, 2048] {
        let mut source = String::new();
        for index in 0..count {
            source.push_str(&format!(
                ":: beat_{index}{}\n> line@{index:020}\n  Dialogue for beat {index}.\n-> {}\n",
                if index == 0 { " default" } else { "" },
                if index + 1 == count {
                    "END".into()
                } else {
                    format!("beat_{}", index + 1)
                }
            ));
        }
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
        let tree = harness.access_tree();
        let mut pending = vec![tree.state().root()];
        let mut nodes = 0;
        while let Some(node) = pending.pop() {
            nodes += 1;
            pending.extend(node.children());
        }
        let scene = writer.map.scene(writer).unwrap();
        assert_eq!(scene.scope.as_ref().unwrap().node_indices.len(), 80);
        assert_eq!(scene.projection.graph.blocks.len(), count);
        // Bound the visible neighbourhood together with its native navigation controls.
        assert!(
            nodes <= 240,
            "bounded Map mounted {nodes} accessibility nodes"
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
            .overview_bounds(middle, 1.)
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
