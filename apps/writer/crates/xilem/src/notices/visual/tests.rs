use super::super::*;
use super::SOURCE;
use crate::{presentation::Theme, tests::support};
use recite_config::WriterPresentationField;
use std::{fs, sync::Arc};
use xilem::{
    ViewCtx,
    core::View,
    masonry::{
        core::NewWidget,
        testing::{TestHarness, TestHarnessParams},
        widgets::SizedBox,
    },
};

#[test]
fn notice_panel_mounts_one_notice_independently_of_retained_notice_count() {
    for (scale, width) in [(100, 1280), (200, 1280), (200, 760)] {
        let mut measured = Vec::new();
        for count in [1_026, 10_260] {
            let directory = support::project(SOURCE).unwrap();
            let path = directory.path().join("scene.recite");
            let mut writer = support::open(&directory).unwrap();
            writer.config.writer.presentation = writer
                .config
                .writer
                .presentation
                .with_value(WriterPresentationField::UiScale, scale)
                .unwrap();
            fs::write(&path, SOURCE.replace("Hello.", "External.")).unwrap();
            for index in 0..count {
                let evidence = writer
                    .session
                    .project()
                    .unwrap()
                    .files()
                    .external_check(&path)
                    .unwrap()
                    .observe();
                writer.notices.found.insert(
                    Target::Source(directory.path().join(format!("retained-{index:05}.recite"))),
                    Evidence::Source(evidence),
                );
            }
            writer.viewport.width = f64::from(width);
            let mut context = ViewCtx::new(
                Arc::new(support::NoProxy),
                Arc::new(tokio::runtime::Runtime::new().unwrap()),
            );
            let view = panel(&writer).unwrap();
            let (pod, _) = view.build(&mut context, &mut writer);
            let mut harness = TestHarness::create_with(
                Theme::new(&writer.config.writer).initial_properties(),
                NewWidget::new(SizedBox::new(pod.new_widget)),
                TestHarnessParams::default()
                    .with_size((width, 700))
                    .with_background(writer.palette.canvas),
            );
            let mut pending = vec![harness.root_widget().as_dyn()];
            let mut nodes = 0;
            while let Some(widget) = pending.pop() {
                nodes += 1;
                pending.extend(widget.children());
            }
            measured.push(nodes);
            let tree = harness.access_tree();
            let mut pending = vec![tree.state().root()];
            let mut compare = false;
            while let Some(node) = pending.pop() {
                compare |=
                    node.label().as_deref() == Some("Compare changed file") && !node.is_disabled();
                pending.extend(node.children());
            }
            assert!(compare);
            let first = writer.notices.selected().unwrap().0.clone();
            writer.notices.next();
            assert!(writer.notices.selected().unwrap().0 != &first);
            if count == 1_026
                && let Ok(prefix) = std::env::var("RECITE_CAPTURE_NOTICES")
            {
                harness
                    .render()
                    .save(format!("{prefix}-panel-{scale}-{width}.png"))
                    .unwrap();
            }
        }
        eprintln!("notices {scale}% {width}px mounted nodes: {measured:?}");
        assert_eq!(measured[0], measured[1]);
        assert!(measured.iter().all(|nodes| *nodes <= 20));
    }
}
