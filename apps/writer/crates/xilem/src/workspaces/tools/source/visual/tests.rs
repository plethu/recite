use super::*;
use crate::presentation::Theme;
use std::sync::Arc;
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
    Unbound,
    Saved,
    Recovered,
}
impl Stage {
    fn name(self) -> &'static str {
        match self {
            Self::Unbound => "unbound",
            Self::Saved => "saved",
            Self::Recovered => "recovered",
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

#[test]
fn declaration_source_binding_recovery_and_preview_render_at_personal_scales() {
    for scale in [100, 200] {
        for stage in [Stage::Unbound, Stage::Saved, Stage::Recovered] {
            let (directory, mut writer, mut jobs) = fixture();
            writer.config.writer.presentation = writer
                .config
                .writer
                .presentation
                .with_value(recite_config::WriterPresentationField::UiScale, scale)
                .unwrap();
            if !matches!(stage, Stage::Unbound) {
                let path = directory.path().join("schema.toml");
                let source = fs::read_to_string(&path).unwrap();
                fs::write(
                    &path,
                    format!("{source}\n# {}\n", "author notes 演劇 ".repeat(2000)),
                )
                .unwrap();
                writer.source_action(SourceAction::Bind(path));
                finish(&mut writer, &mut jobs);
            }
            if matches!(stage, Stage::Recovered) {
                writer
                    .declaration_session_mut()
                    .unwrap()
                    .source
                    .as_mut()
                    .unwrap()
                    .set_draft("# Recovered draft\nunfinished TOML [".into())
                    .unwrap();
                writer.source_action(SourceAction::ExportDraft);
                finish(&mut writer, &mut jobs);
            }
            writer.show_declaration_source();
            writer.workspaces.declarations.source.toggle_preview();
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
            if let Ok(prefix) = std::env::var("RECITE_CAPTURE_DECLARATION_SOURCE") {
                let stem = format!("{prefix}-source-{scale}-{}", stage.name());
                harness.render().save(format!("{stem}.png")).unwrap();
                if !matches!(stage, Stage::Unbound) {
                    if matches!(stage, Stage::Recovered) {
                        reveal(&mut harness, "Import repaired declaration draft");
                        harness.render().save(format!("{stem}-import.png")).unwrap();
                    }
                    reveal(&mut harness, "More source preview");
                    harness
                        .render()
                        .save(format!("{stem}-preview.png"))
                        .unwrap();
                }
            }
        }
    }
}
