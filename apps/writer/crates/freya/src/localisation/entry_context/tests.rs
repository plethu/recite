use super::*;
use crate::tests::support;
use freya_testing::prelude::*;

#[test]
fn failed_source_navigation_preserves_staged_inputs_and_success_opens_preview_without_restart() {
    fn app() -> impl IntoElement {
        let mut writer = support::writer();
        use_hook(move || {
            writer.trial.locale.set("de".into());
            writer.trial.variant.set("earlier".into());
            writer.localisation.write().active = true;
            let pane = *writer.pane.peek();
            let completed = preview_completed(writer, "fr".into(), "formal".into());
            completed.call(Err("Source navigation cancelled".into()));
            assert_eq!(*writer.trial.locale.peek(), "de");
            assert_eq!(*writer.trial.variant.peek(), "earlier");
            assert!(*writer.pane.peek() == pane);
            assert!(writer.localisation.peek().active);
            completed.call(Ok(()));
            assert_eq!(*writer.trial.locale.peek(), "fr");
            assert_eq!(*writer.trial.variant.peek(), "formal");
            assert!(*writer.pane.peek() == Pane::Preview);
            assert!(!writer.localisation.peek().active);
            assert!(
                writer
                    .buffers
                    .model
                    .peek()
                    .as_ref()
                    .unwrap()
                    .preview_page()
                    .is_none()
            );
        });
        label().text("Preview inputs follow successful source navigation")
    }
    let _ = TestingRunner::new(app, (900., 650.).into(), |_| {}, 1.);
}
