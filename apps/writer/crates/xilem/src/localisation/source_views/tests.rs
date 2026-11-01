use super::*;
use crate::presentation::Theme;
use recite_config::WriterPresentationField;
use recite_core::po::PoDocument;
use recite_writer_session::localisation::catalogue::RefreshPreview;
use std::sync::Arc;
use xilem::{
    ViewCtx,
    core::View as _,
    masonry::{
        accesskit::Role,
        testing::{TestHarness, TestHarnessParams},
    },
};

#[test]
fn source_review_and_language_selection_render_at_personal_scales() {
    for scale in [100, 200] {
        for stage in ["languages", "selected-language", "source-review"] {
            let (_directory, mut writer, mut driver) = fixture();
            writer.config.writer.presentation = writer
                .config
                .writer
                .presentation
                .with_value(WriterPresentationField::UiScale, scale)
                .unwrap();
            if stage == "source-review" {
                writer.session.edit(|model| {
                    model.set_draft("The last tram crosses the harbour; this source is applied but remains unsaved.".into());
                    model.apply()
                }).unwrap();
                writer.review_catalogue_sources();
                driver.finish(&mut writer);
            } else {
                writer.new_catalogue();
                writer.localisation.creation.query = "fr".into();
                writer.search_catalogue_languages();
                if stage == "selected-language" {
                    writer.localisation.creation.selected = "fr-CA".into();
                }
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
            if let Ok(prefix) = std::env::var("RECITE_CAPTURE_CATALOGUE_SOURCES") {
                let stem = format!("{prefix}-{scale}-{stage}");
                harness.render().save(format!("{stem}.png")).unwrap();
                super::visual::reveal(
                    &mut harness,
                    if stage == "source-review" {
                        "Apply source updates to PO"
                    } else {
                        "Create catalogue"
                    },
                );
                harness
                    .render()
                    .save(format!("{stem}-actions.png"))
                    .unwrap();
            }
        }
    }
}

#[test]
fn source_refresh_review_cost_depends_on_visible_page_not_entries_or_source_length() {
    let mut measurements = Vec::new();
    for (count, length) in [(1_026, 10_000), (10_260, 200_000)] {
        let (_directory, mut writer, _driver) = fixture();
        let mut before = String::new();
        let mut after = String::new();
        for index in 0..count {
            before.push_str(&format!(
                "msgctxt \"line_{index}\"\nmsgid \"Original {index}\"\nmsgstr \"Translated\"\n\n"
            ));
            let source = if index == 0 {
                "x".repeat(length)
            } else {
                format!("Updated {index}")
            };
            after.push_str(&format!(
                "msgctxt \"line_{index}\"\nmsgid \"{source}\"\nmsgstr \"\"\n\n"
            ));
        }
        let owner = writer.localisation.slot.owner_mut().unwrap();
        owner.catalogue.document = Arc::new(PoDocument::parse(before).unwrap());
        let preview = RefreshPreview::prepare(&owner.catalogue, after).unwrap();
        assert_eq!(preview.changes.len(), count);
        let refresh = refresh::Refresh::new(preview);
        assert!(refresh.pages.iter().all(|page| {
            page.source
                .before
                .as_ref()
                .is_none_or(|value| value.chars().count() <= 1_000)
                && page
                    .source
                    .after
                    .as_ref()
                    .is_none_or(|value| value.chars().count() <= 1_000)
                && page
                    .detail
                    .as_ref()
                    .is_none_or(|value| value.chars().count() <= 1_000)
        }));
        writer.localisation.refresh = Some(refresh);
        writer.localisation.destination = Destination::Refresh;
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
        let (mut nodes, mut apply) = (0, 0);
        while let Some(node) = pending.pop() {
            nodes += 1;
            apply += usize::from(
                node.role() == Role::Button
                    && node.label().as_deref() == Some("Apply source updates to PO"),
            );
            pending.extend(node.children());
        }
        assert_eq!(apply, 1);
        measurements.push(nodes);
        println!(
            "{count} changes / {length} source characters: {nodes} mounted nodes, one Apply action"
        );
    }
    assert_eq!(measurements[0], measurements[1]);
}
