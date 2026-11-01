use super::*;
use crate::{presentation::Theme, workspaces::Open};
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
    Review,
    Busy,
    Retry,
    Reopen,
}
impl Stage {
    fn name(self) -> &'static str {
        match self {
            Self::Review => "review",
            Self::Busy => "busy",
            Self::Retry => "retry",
            Self::Reopen => "reopen",
        }
    }
    fn prepare(
        self,
        writer: &mut Writer,
        jobs: &mut mpsc::UnboundedReceiver<Job>,
        root: &std::path::Path,
    ) {
        review(writer, jobs);
        writer.open_project_workspace(Open::Rename);
        match self {
            Self::Review => (),
            Self::Busy => writer.project_action(Action::ApplyRename),
            Self::Retry => {
                fs::create_dir(root.join("scene.recite.recite-editor-recovery.json")).unwrap();
                writer.project_action(Action::ApplyRename);
                super::super::super::tests::finish(writer, jobs);
            }
            Self::Reopen => {
                writer.project_action(Action::ApplyRename);
                let job = jobs.try_recv().unwrap();
                let ToolWork::Project(work) = job.work else {
                    panic!("exclusive work");
                };
                let completed = work.execute_with(|_, _, _| panic!("injected worker panic"));
                drop(writer.accept_tool(ToolCompleted {
                    request: job.request,
                    result: Ok(Value::Project(Box::new(completed))),
                }));
            }
        }
    }
}
fn reveal<W: Widget>(harness: &mut TestHarness<W>, label: &str) {
    let mut pending = vec![harness.root_widget().as_dyn()];
    while let Some(widget) = pending.pop() {
        let text_matches = widget
            .downcast::<masonry::widgets::TextArea<false>>()
            .is_some_and(|text| text.text() == label);
        let label_matches = widget
            .downcast::<masonry::widgets::Label>()
            .is_some_and(|text| text.text().as_ref() == label);
        if !widget.ctx().is_stashed()
            && (text_matches
                || label_matches
                || support::native_label(harness, widget.id()).as_deref() == Some(label))
        {
            harness.scroll_into_view(widget.id());
            return;
        }
        pending.extend(widget.children());
    }
    panic!("missing render target {label}");
}
#[test]
fn rename_review_and_recovery_states_render_at_personal_scales() {
    for scale in [100, 200] {
        for stage in [Stage::Review, Stage::Busy, Stage::Retry, Stage::Reopen] {
            let (directory, mut writer, mut jobs) = fixture();
            writer.config.writer.presentation = writer
                .config
                .writer
                .presentation
                .with_value(recite_config::WriterPresentationField::UiScale, scale)
                .unwrap();
            stage.prepare(&mut writer, &mut jobs, directory.path());
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
            if let Ok(prefix) = std::env::var("RECITE_CAPTURE_RENAME") {
                let stem = format!("{prefix}-rename-{scale}-{}", stage.name());
                harness.render().save(format!("{stem}.png")).unwrap();
                reveal(
                    &mut harness,
                    match stage {
                        Stage::Review => "Apply project rename",
                        Stage::Busy => "Cancel project action",
                        Stage::Retry => "Retry project edit recovery",
                        Stage::Reopen => "New beat name",
                    },
                );
                harness.render().save(format!("{stem}-action.png")).unwrap();
                if matches!(stage, Stage::Review) {
                    reveal(&mut harness, "After");
                    harness.render().save(format!("{stem}-change.png")).unwrap();
                    reveal(&mut harness, "-> scene.recite::new\n");
                    harness.render().save(format!("{stem}-value.png")).unwrap();
                }
            }
        }
    }
}
