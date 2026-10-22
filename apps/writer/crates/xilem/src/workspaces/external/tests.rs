use crate::{presentation::Theme, tests::support, workspaces::tools::tests::finish};
use std::{fs, sync::Arc};
use tokio::sync::mpsc;
use xilem::{
    ViewCtx,
    core::View as _,
    masonry::{
        core::Widget,
        testing::{TestHarness, TestHarnessParams},
        widgets::{Label, TextArea},
    },
};

const SOURCE: &str =
    ":: start default\n> line@11111111111111111111\n  Original manuscript.\n-> END\n";
#[path = "workflow/tests.rs"]
mod workflow;

fn reveal<W: Widget>(harness: &mut TestHarness<W>, caption: &str) {
    let mut pending = vec![harness.root_widget().as_dyn()];
    while let Some(widget) = pending.pop() {
        let matches = widget
            .downcast::<Label>()
            .is_some_and(|label| label.text().as_ref() == caption)
            || widget
                .downcast::<TextArea<false>>()
                .is_some_and(|text| text.text() == caption);
        if !widget.ctx().is_stashed() && matches {
            harness.scroll_into_view(widget.id());
            return;
        }
        pending.extend(widget.children());
    }
    panic!("missing {caption}");
}

#[test]
fn comparison_pages_are_bounded_and_render_pending_draft_and_merge_actions_at_both_scales() {
    for scale in [100, 200] {
        let directory = support::project(SOURCE).unwrap();
        let mut writer = support::open(&directory).unwrap();
        writer.config.writer.presentation = writer
            .config
            .writer
            .presentation
            .with_value(recite_config::WriterPresentationField::UiScale, scale)
            .unwrap();
        writer
            .session
            .edit(|model| {
                model.select(recite_writer_model::View::Passage(
                    "11111111111111111111".into(),
                ))?;
                model.set_draft(
                    "Unapplied manuscript text remains separate from the accepted source.".into(),
                );
                Ok(())
            })
            .unwrap();
        fs::write(
            directory.path().join("scene.recite"),
            SOURCE.replace("Original manuscript.", "Inspected external manuscript."),
        )
        .unwrap();
        let (sender, mut jobs) = mpsc::unbounded_channel();
        writer.workspaces.tools.sender = Some(sender);
        writer.compare_disk_changes();
        finish(&mut writer, &mut jobs);
        writer.export_merge_draft();
        finish(&mut writer, &mut jobs);
        let inspection = writer.workspaces.external.inspection().unwrap();
        assert!(
            inspection
                .changes
                .iter()
                .flat_map(|change| change.before.iter().chain(&change.after))
                .chain(inspection.pending.iter().flatten())
                .all(|page| page.chars().count() <= 1_000)
        );
        let mut context = ViewCtx::new(
            Arc::new(support::NoProxy),
            Arc::new(tokio::runtime::Runtime::new().unwrap()),
        );
        let view = crate::view::writer(&writer);
        let (pod, _) = view.build(&mut context, &mut writer);
        let mut harness = TestHarness::create_with(
            Theme::new(&writer.config.writer).initial_properties(),
            pod.new_widget,
            TestHarnessParams::default()
                .with_size((1280, 800))
                .with_background(writer.palette.canvas),
        );
        let mut pending = vec![harness.root_widget().as_dyn()];
        let mut compared = 0;
        while let Some(widget) = pending.pop() {
            if !widget.ctx().is_stashed()
                && widget.downcast::<Label>().is_some_and(|text| {
                    let text = text.text();
                    text.contains("Original manuscript.")
                        || text.contains("Inspected external manuscript.")
                })
            {
                let bounds = widget
                    .ctx()
                    .window_transform()
                    .transform_rect_bbox(widget.ctx().border_box());
                assert!(bounds.x0 >= writer.navigation_width(), "{bounds:?}");
                assert!(bounds.x1 <= 1280., "{bounds:?}");
                compared += 1;
            }
            pending.extend(widget.children());
        }
        assert_eq!(compared, 2);
        if let Ok(prefix) = std::env::var("RECITE_CAPTURE_EXTERNAL") {
            let stem = format!("{prefix}-external-{scale}");
            harness.render().save(format!("{stem}.png")).unwrap();
            for (name, caption) in [
                (
                    "draft",
                    "Unapplied manuscript text remains separate from the accepted source.",
                ),
                ("source", "Inspected disk version"),
                ("merge", "Import reviewed merge"),
            ] {
                reveal(&mut harness, caption);
                harness.render().save(format!("{stem}-{name}.png")).unwrap();
            }
        }
    }
}

#[test]
fn a_large_changed_line_does_not_grow_the_mounted_comparison() {
    let mut costs = Vec::new();
    for length in [10_000, 200_000] {
        let source = format!("{SOURCE}\n# {}\n", "a".repeat(length));
        let directory = support::project(&source).unwrap();
        let mut writer = support::open(&directory).unwrap();
        fs::write(
            directory.path().join("scene.recite"),
            source.replace(&"a".repeat(length), &"b".repeat(length)),
        )
        .unwrap();
        let (sender, mut jobs) = mpsc::unbounded_channel();
        writer.workspaces.tools.sender = Some(sender);
        writer.compare_disk_changes();
        finish(&mut writer, &mut jobs);
        let inspection = writer.workspaces.external.inspection().unwrap();
        assert_eq!(inspection.changes.len(), (length + 3).div_ceil(1_000));
        assert!(
            inspection
                .changes
                .iter()
                .all(|row| row.before.as_ref().unwrap().chars().count() <= 1_000
                    && row.after.as_ref().unwrap().chars().count() <= 1_000)
        );
        let labels = support::labels(&mut writer).unwrap();
        costs.push((
            labels.len(),
            labels
                .iter()
                .map(|label| label.chars().count())
                .sum::<usize>(),
        ));
    }
    assert_eq!(costs[1].0, costs[0].0, "{costs:?}");
    // Only the displayed total-page count gains digits; mounted text stays at one page.
    assert!(costs[1].1 <= costs[0].1 + 8, "{costs:?}");
}
