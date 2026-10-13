mod support;
use freya::prelude::*;
use freya_testing::prelude::*;

fn has_text(test: &TestingRunner, text: &str) -> bool {
    test.find(|_, element| {
        Label::try_downcast(element).filter(|label| label.text.starts_with(text))
    })
    .is_some()
}
fn select_all(test: &mut TestingRunner) {
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Character("a".into()),
        code: Code::KeyA,
        modifiers: if cfg!(target_os = "macos") {
            Modifiers::META
        } else {
            Modifiers::CONTROL
        },
    });
    test.sync_and_update();
}
#[test]
fn source_shrink_undo_and_redo_keep_neighbourhood_indices_with_their_graph()
-> Result<(), Box<dyn std::error::Error>> {
    let source = recite_writer_model::workload::source(300, 0, 1);
    let mut test = TestingRunner::new(
        recite_writer::regression_app,
        (1440., 1000.).into(),
        |runner| runner.provide_root_context(move || recite_writer::InitialSource(source)),
        1.,
    )
    .0;
    support::click(&mut test, "Map")?;
    assert!(has_text(&test, "Nearby · 80 of 300 beats"));
    support::click(&mut test, "Whole scene")?;
    assert!(has_text(&test, "Whole scene · 300 beats"));
    support::click(&mut test, "Nearby beats")?;
    assert!(has_text(&test, "Nearby · 80 of 300 beats"));
    support::click(&mut test, "Source")?;
    support::click(&mut test, "Source editor")?;
    select_all(&mut test);
    test.write_text(recite_writer_model::workload::source(3, 0, 1));
    test.poll_n(std::time::Duration::from_millis(16), 5);
    support::click(&mut test, "Apply draft")?;
    support::click(&mut test, "Map")?;
    assert!(!has_text(&test, "Nearby ·"));
    // Undo/redo replace the source while the existing map component stays mounted.
    for _ in 0..3 {
        support::click(&mut test, "Undo")?;
        assert!(has_text(&test, "Nearby · 80 of 300 beats"));
        support::click(&mut test, "Redo")?;
        assert!(!has_text(&test, "Nearby ·"));
        support::click(&mut test, "Fit")?;
        assert!(
            test.find(|_, element| Rect::try_downcast(element)
                .filter(|rect| { rect.accessibility.builder.label() == Some("Select Beat 0") }))
                .is_some()
        );
    }
    Ok(())
}
