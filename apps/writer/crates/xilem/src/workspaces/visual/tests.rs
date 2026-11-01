use super::*;
use crate::presentation;
use std::sync::Arc;
use xilem::masonry::{accesskit::Role, core::Widget};
use xilem::{ViewCtx, core::View as _, masonry::testing::TestHarness};

fn capture_detail<W: Widget>(
    harness: &mut TestHarness<W>,
    prefix: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut pending = vec![harness.root_widget().as_dyn()];
    let target = loop {
        let widget = pending.pop().ok_or("declaration detail pager")?;
        if support::native_label(harness, widget.id()).as_deref() == Some("More declaration detail")
        {
            break widget.id();
        }
        pending.extend(widget.children());
    };
    harness.scroll_into_view(target);
    harness
        .render()
        .save(format!("{prefix}-declarations-detail-200.png"))?;
    Ok(())
}

#[test]
fn thousand_declarations_mount_only_one_page_and_project_screens_render_at_personal_scales()
-> Result<(), Box<dyn std::error::Error>> {
    for scale in [100, 200] {
        for action in [Open::Declarations, Open::Build] {
            let directory = fixture()?;
            let speakers = (0..1_026)
                .map(|index| {
                    format!(
                        "\"speaker_{index:04}\":{{\"display_name\":\"Station announcer {index}\"}}"
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            fs::write(
                directory.path().join("schema.json"),
                format!("{{\"schema_version\":1,\"speakers\":{{{speakers}}}}}"),
            )?;
            let manifest = directory.path().join("recite.project.toml");
            fs::write(
                &manifest,
                fs::read_to_string(&manifest)?.replace("mara", "speaker_0000"),
            )?;
            let mut writer = support::open(&directory)?;
            writer.config.writer.presentation = writer
                .config
                .writer
                .presentation
                .with_value(recite_config::WriterPresentationField::UiScale, scale)?;
            let runtime = Arc::new(tokio::runtime::Runtime::new()?);
            let (sender, mut jobs) = mpsc::unbounded_channel();
            writer.navigation.sender = Some(sender);
            writer.navigation.runtime = Some(runtime.handle().clone());
            writer.open_project_workspace(action);
            support::finish_navigation(&mut writer, &mut jobs)?;
            let mut context = ViewCtx::new(Arc::new(support::NoProxy), runtime);
            let view = crate::view::writer(&writer);
            let (pod, _) = view.build(&mut context, &mut writer);
            let mut harness = TestHarness::create_with_size(
                presentation::Theme::new(&writer.config.writer).initial_properties(),
                pod.new_widget,
                (1280, 800),
            );
            let tree = harness.access_tree();
            let mut pending = vec![tree.state().root()];
            let mut rows = 0;
            let mut nodes = 0;
            while let Some(node) = pending.pop() {
                nodes += 1;
                // The retained scene rail is composed beside the tool, with its own bounded page.
                if node.label().as_deref() == Some("Scene navigation") {
                    let mut rail = node.children().collect::<Vec<_>>();
                    let mut navigation_nodes = 0;
                    while let Some(child) = rail.pop() {
                        navigation_nodes += 1;
                        rail.extend(child.children());
                    }
                    assert!(
                        navigation_nodes <= 64,
                        "Scene navigation must remain bounded."
                    );
                    continue;
                }
                if node.role() == Role::Button
                    && let Some(label) = node.label()
                {
                    rows += usize::from(
                        label.starts_with("Declaration · ")
                            || label.starts_with("Selected · speaker_"),
                    );
                }
                pending.extend(node.children());
            }
            if matches!(action, Open::Declarations) {
                assert_eq!(rows, 5);
            }
            assert!(
                nodes <= 128,
                "Project tool pages must keep bounded native controls."
            );
            println!(
                "{} /{scale}%: {nodes} native nodes",
                if matches!(action, Open::Declarations) {
                    "declarations"
                } else {
                    "build"
                }
            );
            if let Ok(prefix) = std::env::var("RECITE_CAPTURE_WORKSPACES") {
                harness.render().save(format!(
                    "{prefix}-{}-{scale}.png",
                    if matches!(action, Open::Declarations) {
                        "declarations"
                    } else {
                        "build"
                    }
                ))?;
                if scale == 200 && matches!(action, Open::Declarations) {
                    capture_detail(&mut harness, &prefix)?;
                }
            }
        }
    }
    Ok(())
}
