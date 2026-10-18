use super::*;
use freya_testing::prelude::*;
use recite_writer_model::Document;

#[test]
fn empty_scene_detection_does_not_project_large_source_documents() {
    fn app() -> impl IntoElement {
        use_provide_context(|| crate::AppMode::Examples);
        use_hook(|| {
            for (source, empty) in [
                ("# No beats yet.\n".to_owned(), true),
                (recite_writer_model::workload::source(10_000, 0, 100), false),
            ] {
                let model =
                    Workbench::from_document(Document::new(&source).unwrap(), Some(View::Source))
                        .unwrap();
                assert!(model.document().cached_script().is_none());
                assert_eq!(has_no_beats(&model), empty);
                assert!(
                    model.document().cached_script().is_none(),
                    "empty-state observation must not build Script"
                );
            }
        });
        label().text("Source observations stay cold")
    }
    let _ = TestingRunner::new(app, (900., 650.).into(), |_| {}, 1.);
}
