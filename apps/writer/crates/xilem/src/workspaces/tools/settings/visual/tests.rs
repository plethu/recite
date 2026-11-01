use super::*;
use crate::presentation::Theme;
use std::sync::Arc;
use xilem::masonry::widgets::TextArea;
use xilem::{
    ViewCtx,
    core::View as _,
    masonry::{
        core::Widget,
        testing::{TestHarness, TestHarnessParams},
    },
};

#[derive(Clone, Copy)]
enum Stage {
    Rejected,
    Imported,
    Unconfirmed,
}
impl Stage {
    fn name(self) -> &'static str {
        match self {
            Self::Rejected => "rejected",
            Self::Imported => "imported",
            Self::Unconfirmed => "unconfirmed",
        }
    }
}
fn reveal<W: Widget>(harness: &mut TestHarness<W>, label: &str) {
    let mut pending = vec![harness.root_widget().as_dyn()];
    let target = loop {
        let widget = pending.pop().unwrap();
        if support::native_label(harness, widget.id()).as_deref() == Some(label) {
            break widget.id();
        }
        pending.extend(widget.children());
    };
    harness.scroll_into_view(target);
}
fn reveal_preview<W: Widget>(harness: &mut TestHarness<W>) {
    let mut pending = vec![harness.root_widget().as_dyn()];
    let mut largest = None;
    while let Some(widget) = pending.pop() {
        if !widget.ctx().is_stashed() && widget.downcast::<TextArea<false>>().is_some() {
            let height = widget.ctx().border_box().height();
            if largest.is_none_or(|(_, size)| height > size) {
                largest = Some((widget.id(), height));
            }
        }
        pending.extend(widget.children());
    }
    harness.scroll_into_view(largest.unwrap().0);
}
#[test]
fn project_settings_review_and_validation_render_at_personal_scales() {
    for scale in [100, 200] {
        for stage in [Stage::Rejected, Stage::Imported, Stage::Unconfirmed] {
            let (directory, mut writer, mut jobs) = fixture();
            writer.config.writer.presentation = writer
                .config
                .writer
                .presentation
                .with_value(recite_config::WriterPresentationField::UiScale, scale)
                .unwrap();
            writer.project_settings_action(SettingsAction::ExportDraft);
            super::super::super::tests::finish(&mut writer, &mut jobs);
            if matches!(stage, Stage::Imported) {
                let path = writer.workspaces.settings.artifact().unwrap().to_owned();
                let source = fs::read_to_string(&path)
                    .unwrap()
                    .replace("trial", "reviewed");
                fs::write(
                    &path,
                    format!("{source}\n# {}\n", "Author notes 演劇 ".repeat(2000)),
                )
                .unwrap();
                import(&mut writer, &mut jobs, path);
            } else if matches!(stage, Stage::Unconfirmed) {
                fs::write(
                    directory.path().join("recite.project.toml"),
                    "malformed manifest [",
                )
                .unwrap();
                writer.project_settings_action(SettingsAction::Reload);
                super::super::super::tests::finish(&mut writer, &mut jobs);
            } else {
                let path = directory.path().join("invalid.toml");
                fs::write(&path, "unfinished TOML [").unwrap();
                writer
                    .workspaces
                    .settings
                    .set_import_path(path.display().to_string());
                import(&mut writer, &mut jobs, path);
            }
            if matches!(stage, Stage::Imported) {
                writer.workspaces.settings.toggle_preview();
            }
            let runtime = Arc::new(tokio::runtime::Runtime::new().unwrap());
            let mut context = ViewCtx::new(Arc::new(support::NoProxy), runtime);
            let view = crate::view::writer(&writer);
            let (pod, _) = view.build(&mut context, &mut writer);
            let mut harness = TestHarness::create_with(
                Theme::new(&writer.config.writer).initial_properties(),
                pod.new_widget,
                TestHarnessParams::default()
                    .with_size((1280, 800))
                    .with_background(writer.palette.canvas),
            );
            if let Ok(prefix) = std::env::var("RECITE_CAPTURE_PROJECT_SETTINGS") {
                let stem = format!("{prefix}-settings-{scale}-{}", stage.name());
                harness.render().save(format!("{stem}.png")).unwrap();
                reveal(&mut harness, "Import settings draft");
                harness.render().save(format!("{stem}-import.png")).unwrap();
                if matches!(stage, Stage::Imported) {
                    reveal(&mut harness, "More settings preview");
                    harness
                        .render()
                        .save(format!("{stem}-preview.png"))
                        .unwrap();
                    reveal_preview(&mut harness);
                    harness.render().save(format!("{stem}-body.png")).unwrap();
                }
            }
        }
    }
}
