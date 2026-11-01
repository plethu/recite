use super::*;
use crate::presentation::Theme;
use recite_config::WriterPresentationField;
use std::sync::Arc;
use xilem::{
    ViewCtx,
    core::View as _,
    masonry::{
        core::NewWidget,
        testing::{TestHarness, TestHarnessParams},
        widgets::SizedBox,
    },
};

#[test]
fn source_passage_mounts_one_complete_text_page_at_personal_scales() {
    for (scale, width) in [(100, 1280), (200, 1280), (200, 760)] {
        let mut measurements = Vec::new();
        for length in [2_000, 20_000] {
            let (_directory, mut writer, _driver) = fixture();
            let text = "À la gare, le dernier tram attend. ".repeat(length / 33);
            writer
                .session
                .edit(|model| {
                    model.set_draft(text.clone());
                    model.apply()
                })
                .unwrap();
            writer
                .follow_location(&Location {
                    screen: LinkedScreen::Localise,
                    catalogue: Some("fr.po".into()),
                    passage: Some("ffee6p8".into()),
                    ..Location::default()
                })
                .unwrap();
            let passage = writer.localisation.passage.as_ref().unwrap().rows[0]
                .passage()
                .unwrap();
            assert_eq!(passage.pages.concat(), text);
            assert!(
                passage
                    .pages
                    .iter()
                    .all(|page| page.chars().count() <= 1_000)
            );
            writer.config.writer.presentation = writer
                .config
                .writer
                .presentation
                .with_value(WriterPresentationField::UiScale, scale)
                .unwrap();
            let mut context = ViewCtx::new(
                Arc::new(support::NoProxy),
                Arc::new(tokio::runtime::Runtime::new().unwrap()),
            );
            let view = crate::localisation::panel(&writer);
            let (pod, _) = view.build(&mut context, &mut writer);
            let mut harness = TestHarness::create_with(
                Theme::new(&writer.config.writer).initial_properties(),
                NewWidget::new(SizedBox::new(pod.new_widget)),
                TestHarnessParams::default()
                    .with_size((width, 800))
                    .with_background(writer.palette.canvas),
            );
            let tree = harness.access_tree();
            let mut pending = vec![tree.state().root()];
            let (mut nodes, mut next) = (0, 0);
            while let Some(node) = pending.pop() {
                nodes += 1;
                next += usize::from(node.label().as_deref() == Some("More source passage"));
                pending.extend(node.children());
            }
            assert_eq!(next, 1);
            measurements.push(nodes);
            println!("{length} source characters at {scale}%/{width}px: {nodes} nodes");
            if length == 2_000
                && let Ok(prefix) = std::env::var("RECITE_CAPTURE_LINKED_PASSAGE")
            {
                harness
                    .render()
                    .save(format!("{prefix}-{scale}-{width}.png"))
                    .unwrap();
            }
        }
        assert_eq!(measurements[0], measurements[1]);
    }
}
