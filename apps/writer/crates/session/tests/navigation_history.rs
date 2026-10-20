use recite_writer_session::location::{Location, history::History};

fn beat(name: &str) -> Location {
    Location {
        document: "scene.recite".into(),
        beat: Some(name.into()),
        ..Location::default()
    }
}

#[test]
fn failed_steps_preserve_history_and_new_places_replace_only_the_forward_branch() {
    let mut history = History::default();
    history.record(beat("first"));
    history.record(beat("second"));
    let back = history.step(false).unwrap();
    assert_eq!(back.location().beat.as_deref(), Some("first"));
    assert!(history.step(true).is_none());
    assert!(history.accept(back, beat("first")));
    let forward = history.step(true).unwrap();
    history.record(beat("third"));
    assert!(!history.accept(forward, beat("second")));
    assert!(history.step(true).is_none());
    assert_eq!(
        history.step(false).unwrap().location().beat.as_deref(),
        Some("first")
    );
}

#[test]
fn query_updates_coalesce_and_history_retains_a_bounded_number_of_places() {
    let mut history = History::default();
    history.record(beat("first"));
    history.record(Location {
        query: "new query".into(),
        page: 4,
        ..beat("first")
    });
    assert!(history.step(false).is_none());
    history.record(beat("second"));
    assert_eq!(history.step(false).unwrap().location().query, "new query");
    for index in 0..1024 {
        history.record(beat(&format!("beat_{index}")));
    }
    let mut retained = 0;
    while let Some(step) = history.step(false) {
        let target = step.location().clone();
        assert!(history.accept(step, target));
        retained += 1;
    }
    assert_eq!(retained, 127);
}
