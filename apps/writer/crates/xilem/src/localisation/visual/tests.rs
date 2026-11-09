use super::*;
use crate::presentation::ShapedText;
use crate::presentation::Theme;
use recite_config::WriterPresentationField;
use std::sync::Arc;
use xilem::{
    ViewCtx,
    core::View as _,
    masonry::{
        accesskit::Role,
        core::Widget,
        testing::{TestHarness, TestHarnessParams},
        widgets::{Label, TextArea},
    },
};

pub(super) fn reveal<W: Widget>(harness: &mut TestHarness<W>, caption: &str) {
    let mut pending = vec![harness.root_widget().as_dyn()];
    while let Some(widget) = pending.pop() {
        let text = widget
            .downcast::<TextArea<false>>()
            .is_some_and(|text| text.text() == caption);
        let label = widget
            .downcast::<Label>()
            .is_some_and(|text| text.text().as_ref() == caption)
            || widget
                .downcast::<ShapedText>()
                .is_some_and(|text| text.text().as_ref() == caption);
        if !widget.ctx().is_stashed()
            && (text
                || label
                || support::native_label(harness, widget.id()).as_deref() == Some(caption))
        {
            harness.scroll_into_view(widget.id());
            return;
        }
        pending.extend(widget.children());
    }
    panic!("missing render target {caption}");
}

#[test]
fn catalogue_queue_entry_busy_and_close_render_at_personal_scales() {
    for scale in [100, 200] {
        for stage in [
            "queue",
            "entry",
            "large-reading",
            "busy",
            "close",
            "comparison",
        ] {
            let (_directory, mut writer, mut driver) = fixture();
            writer.config.writer.presentation = writer
                .config
                .writer
                .presentation
                .with_value(WriterPresentationField::UiScale, scale)
                .unwrap();
            if stage == "large-reading" {
                writer.config.writer.presentation = writer
                    .config
                    .writer
                    .presentation
                    .with_value(WriterPresentationField::ReadingSize, 32)
                    .unwrap();
            }
            let key = key(&writer);
            writer.update_translation(
                &key,
                0,
                "Le dernier tram attend. Une traduction en cours, à relire avant publication."
                    .into(),
            );
            match stage {
                "queue" => {
                    writer.search_catalogue();
                    driver.finish(&mut writer);
                }
                "busy" => writer.search_catalogue(),
                "close" => writer.request_close(),
                "comparison" => {
                    let path = writer.localisation.slot.path().unwrap();
                    fs::write(path, PO.replace("Bonjour.", "Le dernier tram est parti.")).unwrap();
                    writer.compare_catalogue();
                    driver.finish(&mut writer);
                }
                _ => (),
            }
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
            if let Ok(prefix) = std::env::var("RECITE_CAPTURE_LOCALISATION") {
                let stem = format!("{prefix}-{scale}-{stage}");
                harness.render().save(format!("{stem}.png")).unwrap();
                reveal(
                    &mut harness,
                    match stage {
                        "queue" => "Translation queue",
                        "busy" => "Cancel catalogue action",
                        "close" => "Close and keep recovery",
                        "comparison" => "Keep translation drafts",
                        _ => "Translation",
                    },
                );
                harness.render().save(format!("{stem}-detail.png")).unwrap();
                if stage == "comparison" {
                    reveal(
                        &mut harness,
                        "Discards current translation drafts and loads the inspected PO file.",
                    );
                    harness
                        .render()
                        .save(format!("{stem}-decisions.png"))
                        .unwrap();
                }
            }
        }
    }
}

#[test]
fn optional_entry_and_comparison_contents_keep_stable_native_stack_slots_during_rebuild() {
    let (_directory, mut writer, mut driver) = fixture();
    let mut context = ViewCtx::new(
        Arc::new(support::NoProxy),
        Arc::new(tokio::runtime::Runtime::new().unwrap()),
    );
    let mut view = xilem::view::sized_box(view::panel(&writer));
    let (pod, mut local) = view.build(&mut context, &mut writer);
    let mut harness = TestHarness::create_with_size(
        Theme::new(&writer.config.writer).initial_properties(),
        pod.new_widget,
        (1280, 800),
    );
    for stage in 0..4 {
        match stage {
            0 => writer.search_catalogue(),
            1 => writer.compare_catalogue(),
            2 => writer.resolve_catalogue(ExternalDecision::KeepDrafts),
            _ => {
                let key = EntryKey {
                    catalogue: writer.localisation.slot.identity().unwrap().clone(),
                    id: writer.localisation.queue.rows[0].id,
                };
                writer.inspect_translation(&key);
            }
        }
        driver.finish(&mut writer);
        let next = xilem::view::sized_box(view::panel(&writer));
        harness.edit_root_widget(|root| {
            next.rebuild(&view, &mut local, &mut context, root, &mut writer)
        });
        harness.redraw();
        let expected = match stage {
            1 => "Catalogue disk comparison",
            3 => "Source passage",
            _ => "Translation queue",
        };
        reveal(&mut harness, expected);
        view = next;
    }
}

#[test]
fn catalogue_size_and_long_sources_do_not_increase_mounted_queue_cost() {
    let mut measurements = Vec::new();
    for (count, source_length) in [(1_026, 10_000), (10_260, 200_000)] {
        let (directory, mut writer, mut driver) = fixture();
        let mut po = String::new();
        for index in 0..count {
            let source = if index == 0 {
                "x".repeat(source_length)
            } else {
                format!("Source {index:06}")
            };
            po.push_str(&format!(
                "msgctxt \"line_{index}\"\nmsgid \"{source}\"\nmsgstr \"Translation\"\n\n"
            ));
        }
        fs::write(directory.path().join("fr.po"), po).unwrap();
        writer.catalogue_action(work::Action::Reload);
        driver.finish(&mut writer);
        assert_eq!(writer.localisation.queue.total, count);
        assert_eq!(writer.localisation.queue.rows.len(), 8);
        assert!(
            writer
                .localisation
                .queue
                .rows
                .iter()
                .all(|row| row.source.chars().count() <= 160 && row.caption.chars().count() <= 160)
        );
        let mut context = ViewCtx::new(
            Arc::new(support::NoProxy),
            Arc::new(tokio::runtime::Runtime::new().unwrap()),
        );
        let view = crate::view::writer(&writer);
        let (pod, _) = view.build(&mut context, &mut writer);
        let harness = TestHarness::create_with_size(
            Theme::new(&writer.config.writer).initial_properties(),
            pod.new_widget,
            (1280, 800),
        );
        let tree = harness.access_tree();
        let mut pending = vec![tree.state().root()];
        let (mut nodes, mut rows) = (0, 0);
        while let Some(node) = pending.pop() {
            nodes += 1;
            rows += usize::from(
                node.role() == Role::Button
                    && node
                        .label()
                        .is_some_and(|label| label.starts_with("Open translation · ")),
            );
            pending.extend(node.children());
        }
        assert_eq!(rows, 8);
        measurements.push(nodes);
        println!(
            "{count} entries / {source_length} source characters: {nodes} mounted native nodes, {rows} queue rows"
        );
    }
    assert_eq!(
        measurements[0], measurements[1],
        "native queue cost must depend on the page, not the catalogue or source length"
    );
}
