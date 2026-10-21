use super::*;
use crate::tests::support;
use freya_testing::prelude::*;
use recite_config::WriterView;

#[derive(Clone)]
struct Scenario {
    target: View,
    draft: String,
    preference: WriterView,
}

fn restored_scene() -> impl IntoElement {
    let scenario = consume_root_context::<Scenario>();
    let mut exposed = consume_root_context::<State<Option<Writer>>>();
    let writer = support::writer();
    use_hook(move || {
        let mut original = Workbench::new(support::SOURCE).unwrap();
        original.select(scenario.target).unwrap();
        original.set_draft(scenario.draft);
        let recovery = original.recovery();
        let mut restored = Workbench::new(recovery.source()).unwrap();
        recovery.restore(&mut restored).unwrap();
        writer.buffers.install(restored, false);
        writer.scene_opened(scenario.preference).unwrap();
        exposed.set(Some(writer));
    });
    rect()
}

#[test]
fn restored_prose_stays_editable_without_navigation_or_implicit_apply() {
    for preference in [WriterView::Script, WriterView::Map, WriterView::Source] {
        for draft in ["Recovered prose.", "> syntax\n  Keep this rejected draft."] {
            let scenario = Scenario {
                target: View::Passage(support::A.into()),
                draft: draft.into(),
                preference,
            };
            let (_test, exposed) = TestingRunner::new(
                restored_scene,
                (900., 700.).into(),
                move |runner| {
                    runner.provide_root_context(move || scenario);
                    runner.provide_root_context(|| State::create(None::<Writer>))
                },
                1.,
            );
            let writer = exposed.peek().unwrap();
            let state = writer.buffers.model.peek();
            let model = state.as_ref().unwrap();
            assert_eq!(model.draft(), draft);
            assert_eq!(model.view(), &View::Passage(support::A.into()));
            assert_eq!(model.document().source(), support::SOURCE);
            assert_eq!(*writer.layout.view.peek(), WriterView::Script);
            assert!(matches!(*writer.pane.peek(), Pane::Script));
            assert_eq!(*writer.selection.peek(), Some("start".into()));
        }
    }
}

#[test]
fn restored_source_stays_in_source_regardless_of_saved_view_preference() {
    for preference in [WriterView::Script, WriterView::Map, WriterView::Source] {
        let scenario = Scenario {
            target: View::Source,
            draft: "Incomplete source ->".into(),
            preference,
        };
        let (_test, exposed) = TestingRunner::new(
            restored_scene,
            (900., 700.).into(),
            move |runner| {
                runner.provide_root_context(move || scenario);
                runner.provide_root_context(|| State::create(None::<Writer>))
            },
            1.,
        );
        let writer = exposed.peek().unwrap();
        let state = writer.buffers.model.peek();
        let model = state.as_ref().unwrap();
        assert_eq!(model.draft(), "Incomplete source ->");
        assert_eq!(model.view(), &View::Source);
        assert_eq!(model.document().source(), support::SOURCE);
        assert_eq!(*writer.layout.view.peek(), WriterView::Source);
        assert!(matches!(*writer.pane.peek(), Pane::Map));
    }
}
