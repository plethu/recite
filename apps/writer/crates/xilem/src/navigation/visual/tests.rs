use super::*;
use crate::tests::support::NoProxy;
use xilem::masonry::accesskit;
use xilem::{
    ViewCtx,
    core::View,
    masonry::{core::NewWidget, testing::TestHarness, widgets::SizedBox},
};

#[test]
fn pending_navigation_and_reload_keep_manuscript_editable_and_expose_cancel_while_save_is_disabled()
{
    for reload in [false, true] {
        check_pending_navigation(reload);
    }
}

fn check_pending_navigation(reload: bool) {
    for scale in [100, 200] {
        let mut fixture = fixture().unwrap();
        let writer = &mut fixture.writer;
        let mut config = writer.config.clone();
        config.writer.presentation = config
            .writer
            .presentation
            .with_value(recite_config::WriterPresentationField::UiScale, scale)
            .unwrap();
        writer.configure(config);
        if reload {
            writer.reload();
        } else {
            writer.switch(&fixture.other);
        }
        let mut ctx = ViewCtx::new(
            Arc::new(NoProxy),
            Arc::new(tokio::runtime::Runtime::new().unwrap()),
        );
        let view = crate::view::writer(writer);
        let sender = writer.navigation.sender.clone();
        let (pod, _) = view.build(&mut ctx, writer);
        writer.navigation.sender = sender;
        let defaults = crate::presentation::Theme::new(&writer.config.writer).initial_properties();
        let mut harness = TestHarness::create_with_size(
            defaults,
            NewWidget::new(SizedBox::new(pod.new_widget)),
            (1280, 700),
        );
        let tree = harness.access_tree();
        let mut pending = vec![tree.state().root()];
        let (mut cancel, mut editable, mut save_disabled) = (false, false, false);
        while let Some(node) = pending.pop() {
            cancel |=
                node.label().as_deref() == Some("Cancel pending action") && !node.is_disabled();
            editable |= node.role() == accesskit::Role::MultilineTextInput && !node.is_disabled();
            save_disabled |= node.label().as_deref() == Some("Save file") && node.is_disabled();
            pending.extend(node.children());
        }
        assert!(cancel && editable && save_disabled);
        if let Ok(prefix) = std::env::var("RECITE_CAPTURE_NAVIGATION_WAIT") {
            harness
                .render()
                .save(if reload {
                    format!("{prefix}-reload-{scale}.png")
                } else {
                    format!("{prefix}-{scale}.png")
                })
                .unwrap();
        }
        writer.cancel_navigation();
        finish_navigation(writer, &mut fixture.jobs).unwrap();
        assert_eq!(writer.session.model().document().source(), SOURCE);
    }
}
